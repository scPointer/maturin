//! 用户程序的数据及状态信息
//! 一个 TaskControlBlock 包含了一个任务(或进程)的所有信息

//#![deny(missing_docs)]

use super::{CloneFlags, KernelStack, TaskContext};
use crate::{
    arch::get_cpu_id,
    constants::{NO_PARENT, USER_STACK_OFFSET},
    file::{check_file_exists, FdManager},
    loaders::parse_user_app,
    memory::{new_memory_set_for_task, MemorySet, PTEFlags, Tid, VirtAddr},
    signal::{global_register_signals, SignalHandlers, SignalReceivers, SignalUserContext},
    timer::get_time,
    trap::TrapContext,
};
use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use lock::Mutex;

/// 任务控制块，包含一个用户程序的所有状态信息，但不包括与调度有关的信息。
/// 默认在TCB的外层对其的访问不会冲突，所以外部没有用锁保护，内部的 mutex 仅用来提供可变性
///
/// 目前来说，TCB外层可能是调度器或者 CpuLocal：
/// 1. 如果它在调度器里，则 Scheduler 内部不会修改它，且从 Scheduler 里取出或者放入 TCB 是由调度器外部的 Mutex 保护的；
/// 2. 如果它在 CpuLocal 里，则同时只会有一个核可以访问它，也不会冲突。
pub struct TaskControlBlock {
    /// 用户程序的内核栈，内部包含申请的内存空间
    /// 因为 struct 内部保存了页帧 Frame，所以 Drop 这个结构体时也会自动释放这段内存
    pub kernel_stack: KernelStack,
    /// 进程 id。创建任务时实际分配的是 tid 而不是 pid，所以没有对应的 Pid 结构保护
    pub pid: usize,
    /// 线程 id
    pub tid: Tid,
    /// 当退出时是否向父进程发送信号 SIGCHLD。
    /// 如果创建时带 CLONE_THREAD 选项，则不发送信号，除非它是线程组(即拥有相同pid的所有线程)中最后一个退出的线程；
    /// 否则发送信号
    pub send_sigchld_when_exit: bool,
    /// 信号量对应的一组处理函数。
    /// 因为发送信号是通过 pid/tid 查找的，因此放在 inner 中一起调用时更容易导致死锁
    pub signal_handlers: Arc<Mutex<SignalHandlers>>,
    /// 接收信号的结构。每个线程中一定是独特的，而上面的 handler 可能是共享的
    pub signal_receivers: Arc<Mutex<SignalReceivers>>,
    /// 任务的内存段(内含页表)，同时包括用户态和内核态
    pub vm: Arc<Mutex<MemorySet>>,
    /// 管理进程的所有文件描述符
    pub fd_manager: Arc<Mutex<FdManager>>,
    /// 任务的状态信息
    pub inner: Arc<Mutex<TaskControlBlockInner>>,
}

/// 任务控制块的可变部分
pub struct TaskControlBlockInner {
    /// 用户程序当前的工作目录
    /// - 注意 dir[0] == '.' ，如以 ./ 开头时代表根目录，以 "./abc/" 开头代表根目录下的abc目录。
    /// 这样处理是因为 open_file 时先打开文件所在目录，它的实现是先打开根目录，再从根目录找相对路径
    pub dir: String,
    /// 父进程的 pid。
    /// - 因为拿到 Pid 代表“拥有”这个 id 且 Drop 时会自动释放，所以此处用 usize 而不是 Pid。
    /// - 又因为它可能会在父进程结束时被修改为初始进程，所以是可变的。
    pub ppid: usize,
    /// 进程开始运行的时间
    pub start_tick: usize,
    /// 用户堆的堆顶。
    /// 用户堆和用户栈共用空间，反向增长，即从 USER_STACK_OFFSET 开始往上增加。
    /// 本来不应该由内存记录的，但 brk() 系统调用要用
    pub user_heap_top: usize,
    /// 任务执行状态
    pub task_status: TaskStatus,
    /// 上下文信息，用于切换，包含所有必要的寄存器
    /// 实际在第一次初始化时还包含了用户程序的入口地址和用户栈
    pub task_cx: TaskContext,
    /// 父进程
    pub parent: Option<Weak<TaskControlBlock>>,
    /// 子进程
    pub children: Vec<Arc<TaskControlBlock>>,
    /// sys_exit 时输出的值
    pub exit_code: i32,
    /// 子线程初始化时，存放 tid 的地址。当且仅当创建时包含 CLONE_CHILD_SETTID 才非0
    pub set_child_tid: usize,
    /// 子线程初始化时，将这个地址清空；子线程退出时，触发这里的 futex。
    /// 在创建时包含 CLONE_CHILD_SETTID 时才非0，但可以被 sys_set_tid_address 修改
    pub clear_child_tid: usize,
    /// 处理信号时，保存的之前的用户线程的上下文信息
    trap_cx_before_signal: Option<TrapContext>,
    /// 保存信息时，处理函数是否设置了 SIGINFO 选项
    /// 如果设置了，说明信号触发前的上下文信息通过 ucontext 传递给了用户，
    /// 此时用户可能修改其中的 pc 信息(如musl-libc 的 pthread_cancel 函数)。
    /// 在这种情况下，需要手动在 sigreturn 时更新已保存的上下文信息
    signal_set_siginfo: bool,
}

unsafe impl Send for TaskControlBlockInner {}

impl TaskControlBlock {
    /// 从用户程序名生成 TCB，其中文件名默认为 args[0]
    ///
    /// 在目前的实现下，如果生成 TCB 失败，只有以下情况：
    /// 1. 找不到文件名所对应的文件
    /// 2. 或者 loader 解析失败
    ///
    /// 才返回 None，其他情况下生成失败会 Panic。
    /// 因为上面这两种情况是用户输入可能带来的，要把结果反馈给用户程序；
    /// 而其他情况(如 pid 分配失败、内核栈分配失败)是OS自己出了问题，应该停机。
    ///
    /// 目前只有初始进程(/task/mod.rs: ORIGIN_USER_PROC) 直接通过这个函数初始化，
    /// 其他进程应通过 clone / exec 生成
    pub fn from_app_name(app_dir: &str, ppid: usize, args: Vec<String>) -> Option<Self> {
        if args.len() < 1 {
            // 需要至少有一项指定文件名
            return None;
        }
        let app_name_string: String = args[0].clone();
        let app_name = app_name_string.as_str();
        if !check_file_exists(app_dir, app_name) {
            return None;
        }
        // 新建页表，包含内核段
        let mut vm = new_memory_set_for_task().unwrap();
        // 找到用户名对应的文件，将用户地址段信息插入页表和 VmArea
        parse_user_app(app_dir, app_name, &mut vm, args)
            .map(|(user_entry, user_stack)| {
                //println!("user MemorySet {:#x?}", vm);
                // 初始化内核栈，它包含关于进入用户程序的所有信息
                let kernel_stack = KernelStack::new().unwrap();
                //kernel_stack.print_info();
                let tid = Tid::new().unwrap();
                let pid = tid.0;
                let stack_top = kernel_stack
                    .push_first_context(TrapContext::app_init_context(user_entry, user_stack));
                let signal_handlers = Arc::new(Mutex::new(SignalHandlers::new()));
                let signal_receivers = Arc::new(Mutex::new(SignalReceivers::new()));
                global_register_signals(tid.0, signal_receivers.clone());
                //println!("tid = {}", tid.0);
                TaskControlBlock {
                    kernel_stack: kernel_stack,
                    pid: pid,
                    tid: tid,
                    send_sigchld_when_exit: true,
                    signal_handlers: signal_handlers,
                    signal_receivers: signal_receivers,
                    vm: Arc::new(Mutex::new(vm)),
                    fd_manager: Arc::new(Mutex::new(FdManager::new())),
                    inner: Arc::new(Mutex::new(TaskControlBlockInner {
                        dir: String::from(app_dir),
                        ppid: ppid,
                        start_tick: get_time(),
                        user_heap_top: USER_STACK_OFFSET,
                        task_cx: TaskContext::goto_restore(stack_top),
                        task_status: TaskStatus::Ready,
                        parent: None,
                        children: Vec::new(),
                        exit_code: 0,
                        set_child_tid: 0,
                        clear_child_tid: 0,
                        trap_cx_before_signal: None,
                        signal_set_siginfo: false,
                    })),
                }
            })
            .ok()
    }
    /// 从 clone 系统调用初始化一个TCB，并设置子进程对用户程序的返回值为0。
    ///
    /// - 参数 user_stack 为是否指定用户栈地址。如为 None，则沿用同进程的栈，否则使用该地址。由用户保证这个地址是有效的。
    /// - send_sigchld_when_exit 参见 TaskControlBlock 定义说明
    /// - flags 参见 clone_flags.rs
    /// - tls 为新任务的 tp 值，当包含 CLONE_SETTLS 时设置
    /// - ptid 为当前任务地址空间中的地址，当包含 CLONE_PARENT_SETTID 时，新任务 tid 被存入此处
    /// - ctid 为新任务地址空间中的地址，当包含 CLONE_CHILD_SETTID 时，新任务 tid 被存入此处
    ///
    /// 这里只把父进程内核栈栈底的第一个 TrapContext 复制到子进程，
    /// 所以**必须保证对这个函数的调用是来自用户异常中断，而不是内核异常中断**。因为只有这时内核栈才只有一层 TrapContext。
    pub fn from_clone(
        self: &Arc<TaskControlBlock>,
        user_stack: Option<usize>,
        send_sigchld_when_exit: bool,
        flags: CloneFlags,
        tls: usize,
        ptid: usize,
        ctid: usize,
    ) -> Arc<Self> {
        //println!("start clone");
        let mut inner = self.inner.lock();
        // 是否共享 MemorySet
        let vm = if flags.contains(CloneFlags::CLONE_VM) {
            self.vm.clone()
        } else {
            Arc::new(Mutex::new(self.vm.lock().copy_as_fork().unwrap()))
        };
        // 是否共享文件描述符
        let fd_manager = if flags.contains(CloneFlags::CLONE_FILES) {
            self.fd_manager.clone()
        } else {
            Arc::new(Mutex::new(self.fd_manager.lock().copy_all()))
        };
        // 是否共享信号处理函数
        let new_signal_handlers = if flags.contains(CloneFlags::CLONE_SIGHAND) {
            self.signal_handlers.clone()
        } else {
            // 如果不共享，也要复制整个模块的值，只是不一起更新
            Arc::new(Mutex::new(self.signal_handlers.lock().clone()))
        };
        let tid = Tid::new().unwrap();
        let pid = if flags.contains(CloneFlags::CLONE_THREAD) {
            self.pid
        } else {
            tid.0
        };
        let ppid = if flags.contains(CloneFlags::CLONE_PARENT) {
            inner.ppid
        } else {
            self.tid.0
        };
        let signal_receivers = Arc::new(Mutex::new(SignalReceivers::new()));
        // 存入全局表中的 signals 是只复制指针
        global_register_signals(tid.0, signal_receivers.clone());

        let kernel_stack = KernelStack::new().unwrap();
        // 与 new 方法不同，这里从父进程的 TrapContext 复制给子进程
        let mut trap_context = unsafe { *self.kernel_stack.get_first_context() };
        // 手动设置返回值为0，这样两个进程返回用户时除了返回值以外，都是完全相同的
        trap_context.set_a0(0);
        // 检查是否需要设置 tls
        if flags.contains(CloneFlags::CLONE_SETTLS) {
            trap_context.set_tp(tls);
        }
        // 检查是否在父任务地址中写入 tid
        if flags.contains(CloneFlags::CLONE_PARENT_SETTID) {
            // 有可能这个地址是 lazy alloc 的，需要先检查
            if self.vm.lock().manually_alloc_page(ptid).is_ok() {
                unsafe {
                    *(ptid as *mut i32) = tid.0 as i32;
                }
            }
        }
        if flags.contains(CloneFlags::CLONE_CHILD_SETTID)
            || flags.contains(CloneFlags::CLONE_CHILD_CLEARTID)
        {
            // 复制地址空间时就可以直接在当前地址空间下操作
            if flags.contains(CloneFlags::CLONE_VM) {
                if self.vm.lock().manually_alloc_page(ctid).is_ok() {
                    unsafe {
                        *(ctid as *mut i32) = if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
                            tid.0 as i32
                        } else {
                            0
                        };
                    }
                }
            } else {
                // 否则需要手动查询
                if vm.lock().manually_alloc_page(ctid).is_ok() {
                    if let Some(paddr) = vm.lock().pt.query(ctid) {
                        unsafe {
                            *(paddr as *mut i32) = if flags.contains(CloneFlags::CLONE_CHILD_SETTID)
                            {
                                tid.0 as i32
                            } else {
                                0
                            };
                        }
                    }
                }
            }
        }
        // 设置用户栈
        if let Some(user_stack_pos) = user_stack {
            trap_context.set_sp(user_stack_pos);
            //println!("sepc {:x} stack {:x}", trap_context.sepc, trap_context.get_sp());
        }
        let stack_top = kernel_stack.push_first_context(trap_context);

        let dir = String::from(&inner.dir[..]);
        let new_tcb = Arc::new(TaskControlBlock {
            pid: pid,
            tid: tid,
            send_sigchld_when_exit: send_sigchld_when_exit,
            kernel_stack: kernel_stack,
            signal_handlers: new_signal_handlers,
            signal_receivers: signal_receivers,
            vm: vm,
            fd_manager: fd_manager,
            inner: {
                Arc::new(Mutex::new(TaskControlBlockInner {
                    dir: dir,
                    ppid: ppid,
                    start_tick: get_time(),
                    user_heap_top: USER_STACK_OFFSET,
                    task_cx: TaskContext::goto_restore(stack_top),
                    task_status: TaskStatus::Ready,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    set_child_tid: if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
                        ctid
                    } else {
                        0
                    },
                    clear_child_tid: if flags.contains(CloneFlags::CLONE_CHILD_CLEARTID) {
                        ctid
                    } else {
                        0
                    },
                    trap_cx_before_signal: None,
                    signal_set_siginfo: false,
                }))
            },
        });
        if !flags.contains(CloneFlags::CLONE_PARENT) {
            inner.children.push(new_tcb.clone());
        }
        //println!("end clone");
        new_tcb
    }

    /// 从 exec 系统调用修改当前TCB，**默认新的用户程序与当前程序在同路径下**：
    /// 1. 从 ELF 文件中生成新的 MemorySet 替代当前的
    /// 2. 修改内核栈栈底的第一个 TrapContext 为新的用户程序的入口
    /// 3. 将传入的 args 作为用户程序执行时的参数
    ///
    /// 如找不到对应的用户程序，则不修改当前进程且返回 False。
    ///
    /// 注意 exec 不会清空用户程序执行的时间
    pub fn exec(&self, app_name: &str, args: Vec<String>) -> bool {
        let mut inner = self.inner.lock();
        if !check_file_exists(inner.dir.as_str(), app_name) {
            return false;
        }
        // 清空用户堆
        inner.user_heap_top = USER_STACK_OFFSET;
        // 清空 MemorySet 中用户段的地址
        self.vm.lock().clear_user_and_save_kernel();
        // 清空信号模块
        self.signal_handlers.lock().clear();
        self.signal_receivers.lock().clear();
        // 处理 fd 中需要在 exec 时关闭的文件
        self.fd_manager.lock().close_cloexec_files();
        // 如果用户程序调用时没有参数，则手动加上程序名作为唯一的参数
        // 需要这个调整，是因为用户库(/user下)使用了 rCore 的版本，
        // 里面的 user_shell 调用 exec 时会加上程序名作为 args 的第一个参数
        // 但是其他函数调用 exec 时只会传入空的 args (包括初始进程)
        // 为了鲁棒性考虑，此处不修改用户库，而是手动分别这两种情况
        let args = if args.len() == 0 {
            vec![String::from(app_name)]
        } else {
            args
        };
        for i in 0..args.len() {
            info!("[cpu {}] args[{}] = '{}'", get_cpu_id(), i, args[i]);
        }

        // 然后把新的信息插入页表和 VmArea
        let dir = String::from(&inner.dir[..]);
        let mut self_vm = self.vm.lock();
        parse_user_app(dir.as_str(), app_name, &mut self_vm, args)
            .map(|(user_entry, user_stack)| {
                // 修改完 MemorySet 映射后要 flush 一次
                self_vm.flush_tlb();
                //println!("user vm {:#x?}", inner.vm);
                // argc 和 argv 存在用户栈顶，而按用户库里的实现是需要放在 a0 和 a1 寄存器中，所以这里手动取出
                let argc = unsafe { *(user_stack as *const usize) };
                let argv = unsafe { ((user_stack as *const usize).add(1)) as usize };
                //println!("argc {} argv0 {:x}", argc, argv0);
                // 此处实际上覆盖了 kernel_stack 中原有的 TrapContext，内部用 unsafe 规避了此处原本应有的 mut
                let stack_top =
                    self.kernel_stack
                        .push_first_context(TrapContext::app_exec_context(
                            user_entry, user_stack, argc, argv,
                        ));
                inner.task_cx = TaskContext::goto_restore(stack_top);

                //let trap_context = unsafe {*self.kernel_stack.get_first_context() };
                //println!("sp = {:x}, entry = {:x}, sstatus = {:x}", trap_context.x[2], trap_context.sepc, trap_context.sstatus.bits());
            })
            .is_ok()
    }

    /// 映射一段内存地址到文件或设备。
    ///
    /// anywhere 选项指示是否可以映射到任意位置，一般与 `MAP_FIXED` 关联。
    /// 如果 anywhere=true，则将 start 视为 hint
    pub fn mmap(
        &self,
        start: VirtAddr,
        end: VirtAddr,
        flags: PTEFlags,
        data: &[u8],
        anywhere: bool,
    ) -> Option<usize> {
        //info!("start {} , end {}, data.len {}", start, end, data.len());
        if end - start < data.len() {
            None
        } else {
            self.vm
                .lock()
                .push_with_data(start, end, flags, data, anywhere)
                .ok()
        }
    }
    /// 取消一段内存地址映射
    pub fn munmap(&self, start: VirtAddr, end: VirtAddr) -> bool {
        self.vm.lock().pop(start, end).is_ok()
    }
    /// 修改任务状态
    pub fn set_status(&self, new_status: TaskStatus) {
        let mut inner = self.inner.lock();
        inner.task_status = new_status;
    }
    /// 输入 exit code
    pub fn set_exit_code(&self, exit_code: i32) {
        let mut inner = self.inner.lock();
        inner.exit_code = exit_code;
    }
    /// 读取任务状态
    pub fn get_status(&self) -> TaskStatus {
        let inner = self.inner.lock();
        inner.task_status
    }
    /// 读取任务上下文
    pub fn get_task_cx_ptr(&self) -> *const TaskContext {
        let inner = self.inner.lock();
        &inner.task_cx
    }
    /// 获取 pid 的值
    pub fn get_pid_num(&self) -> usize {
        self.pid
    }
    /// 获取 tid 的值，不会转移或释放 Tid 的所有权
    pub fn get_tid_num(&self) -> usize {
        self.tid.0
    }
    /// 获取 ppid 的值
    pub fn get_ppid(&self) -> usize {
        let ppid = self.inner.lock().ppid;
        if ppid == NO_PARENT {
            1
        } else {
            ppid
        }
    }
    /// 获取程序开始时间
    pub fn get_start_tick(&self) -> usize {
        self.inner.lock().start_tick
    }
    /// 获取用户堆顶地址
    pub fn get_user_heap_top(&self) -> usize {
        self.inner.lock().user_heap_top
    }
    /// 重新设置堆顶地址，如成功则返回设置后的堆顶地址，否则保持不变，并返回之前的堆顶地址。
    /// 新地址需要在用户栈内，并且不能碰到目前的栈
    pub fn set_user_heap_top(&self, new_top: usize) -> usize {
        let user_sp = unsafe { (*self.kernel_stack.get_first_context()).get_sp() };
        let mut inner = self.inner.lock();
        if new_top >= USER_STACK_OFFSET && new_top < user_sp {
            inner.user_heap_top = new_top;
            new_top
        } else {
            inner.user_heap_top
        }
    }
    /// 如果当前进程已是运行结束，则获取其 exit_code，否则返回 None
    pub fn get_code_if_exit(&self) -> Option<i32> {
        let inner = self.inner.try_lock()?;
        match inner.task_status {
            TaskStatus::Zombie => Some(inner.exit_code),
            _ => None,
        }
    }
    /// 设置 clear_child_tid 属性
    pub fn set_tid_address(&self, addr: usize) {
        self.inner.lock().clear_child_tid = addr;
    }
    /// 如果当前没有在信号处理函数中，则保存当前用户上下文信息，返回true。
    /// 否则不保存并返回false
    pub fn save_trap_cx_if_not_handling_signals(&self) -> bool {
        let mut inner = self.inner.lock();
        if inner.trap_cx_before_signal.is_some() {
            return false;
        }
        inner.trap_cx_before_signal = Some(unsafe {
            *self.kernel_stack.get_first_context()
        });
        // 默认没有 SIGINFO，如果有则需要用 save_if_set_siginfo 设置
        inner.signal_set_siginfo = false;
        true
    }
    /// 记录信号处理函数是否设置了 SIGINFO
    pub fn save_if_set_siginfo(&self, signal_set_siginfo: bool) {
        self.inner.lock().signal_set_siginfo = signal_set_siginfo;   
    }
    /// 恢复用户上下文信息，返回true。如没有已保存的上下文信息，则返回 false
    pub fn load_trap_cx_if_handling_signals(&self) -> bool {
        let mut inner = self.inner.lock();
        if let Some(trap_cx_old) = inner.trap_cx_before_signal.take() {
            //info!("sig returned");
            unsafe {
                let trap_cx_now = self.kernel_stack.get_first_context();
                // 这里假定是 sigreturn 触发的，即用户的信号处理函数 return 了(cancel_handler)
                // 也就是说信号触发时的 sp 就是现在的 sp
                let sp = (*trap_cx_now).get_sp();
                // 获取可能被修改的 pc
                let pc = (*(sp as *const SignalUserContext)).get_pc();
                *trap_cx_now = trap_cx_old;
                if inner.signal_set_siginfo { // 更新用户修改的 pc
                    (*trap_cx_now).set_sepc(pc);
                    info!("sig return sp = {:x} pc = {:x}", sp, pc);
                }
            }
            true
        } else {
            false
        }
    }
}

/// 任务执行状态
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// 还未初始化
    UnInit,
    /// 已初始化但还未执行，可以被任意一个核执行
    Ready,
    /// 正在被一个核执行
    Running,
    /// 进程在用户端已退出，但内核端还有些工作要处理，例如把它的所有子进程交给初始进程
    Dying,
    /// 僵尸进程，已退出，但其资源还在等待回收
    Zombie,
    /// 已执行完成
    Exited,
}
