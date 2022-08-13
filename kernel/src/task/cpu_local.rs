//! 每个核当前正在运行的任务及上下文信息

use super::{
    TaskContext, TaskControlBlock, TaskStatus, __move_to_context, __switch,
    fetch_task_from_scheduler, push_task_to_scheduler, ORIGIN_USER_PROC,
};
use crate::{
    arch::get_cpu_id,
    constants::{CPU_ID_LIMIT, IS_TEST_ENV, NO_PARENT, USER_STACK_RED_ZONE},
    error::{OSError, OSResult},
    file::show_testcase_result,
    memory::{enable_kernel_page_table, PTEFlags, VirtAddr},
    signal::{
        global_logoff_signals, send_signal, SigActionDefault, SigActionFlags, SigInfo, SignalNo,
        SignalUserContext, SIG_IGN,
    },
};
use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;
use lock::Mutex;

/// 每个核当前正在运行的任务及上下文信息。
/// 注意，如果一个核没有运行在任何任务上，那么它会回到 idle_task_cx 的上下文，而这里的栈就是启动时的栈。
/// 启动时的栈空间在初始化内核 MemorySet 与页表时有留出 shadow page，也即如果在核空闲时不断嵌套异常中断导致溢出，
/// 会在 trap 中进入 StorePageFault，然后panic终止系统
pub struct CpuLocal {
    /// 这个核当前正在运行的用户程序
    current: Option<Arc<TaskControlBlock>>,
    /// 无任务时的上下文，实际存的是启动时的上下文(其中的栈是 entry.S 中的 idle_stack)
    idle_task_cx: TaskContext,
}

impl CpuLocal {
    /// 创建一个 cpu 相关的信息
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    /// 获取无用户程序状态的内核上下文
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    /// 拿走当前 cpu 正在运行的任务
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    /// 读取当前 cpu 正在运行的任务
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static::lazy_static! {
    /// 所有 CPU 的上下文信息
    pub static ref CPU_CONTEXTS: Vec<Mutex<CpuLocal>> = {
        let mut cpu_contexts: Vec<Mutex<CpuLocal>> = Vec::new();
        for _ in 0..CPU_ID_LIMIT {
            cpu_contexts.push(Mutex::new(CpuLocal::new()));
        }
        cpu_contexts
    };
}

/// 开始执行用户程序
pub fn run_tasks() -> ! {
    let cpu_id = get_cpu_id();
    loop {
        if let Some(task) = fetch_task_from_scheduler() {
            let mut cpu_local = CPU_CONTEXTS[cpu_id].lock();
            //let mut task_inner = task.lock();
            let idle_task_cx_ptr = cpu_local.get_idle_task_cx_ptr();
            let next_task_cx_ptr = task.get_task_cx_ptr();
            task.set_status(TaskStatus::Running);

            let tid = task.get_tid_num();
            info!("[cpu {}] now running on tid = {}", cpu_id, tid);
            //drop(task_inner);
            unsafe {
                task.vm.lock().activate();
            }
            // 标记内核态进入任务的时间
            task.time.lock().switch_into_task();
            cpu_local.current = Some(task);
            // 切换前要手动 drop 掉引用
            drop(cpu_local);
            // 切换到用户程序执行
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
            // 在上面的用户程序中，会执行 suspend_current_and_run_next() 或  exit_current_and_run_next(exit_code: i32)
            // 在其中会修改 current.task_status 和 exit_code，但任务本身还在被当前 CPU 占用，需要下面再将其插入队列或
            let mut cpu_local = CPU_CONTEXTS[cpu_id].lock();
            // 标记内核态退出任务的时间
            cpu_local.current().unwrap().time.lock().switch_out_task();
            // 切换回只有内核的页表。在此之后就不能再访问该任务用户空间的内容
            enable_kernel_page_table();
            // 此时已切回空闲任务
            if let Some(task) = cpu_local.take_current() {
                // println!("[cpu {}] now leave pid = {}", cpu_id, task.get_pid_num());
                let status = task.get_status();
                match status {
                    TaskStatus::Ready => {
                        // 将暂停的用户程序塞回任务队列
                        push_task_to_scheduler(task);
                    }
                    TaskStatus::Dying => {
                        if !IS_TEST_ENV && task.get_pid_num() == 0 {
                            // 这是初始进程，且不在测试环境
                            panic!("origin user proc exited, All applications completed.");
                        } else {
                            handle_zombie_task(&mut cpu_local, task);
                        }
                    }
                    _ => {
                        panic!("invalid task status when switched out");
                    }
                }
            } else {
                panic!("[cpu {}] CpuLocal: switched from empty task", get_cpu_id());
            }
            // 因为 task 是 task_current() 得到的，所以如果 task 不是 ORIGIN_USER_PROC，它在上面的 if 结束时就已经没有了 Arc 引用
            // 其内部的 Pid, MemorySet 等应在此时被 Drop
            drop(cpu_local);
        }
    }
}

/// 暂停当前用户程序，回到 idle 状态
pub fn suspend_current_task() {
    let cpu_id = get_cpu_id();
    let mut cpu_local = CPU_CONTEXTS[cpu_id].lock();
    let task = cpu_local.current().unwrap();
    //let task_inner = task.lock();
    task.set_status(TaskStatus::Ready);
    // let task = cpu_local.take_current_task(); 只有写好用户程序的内核栈、回到 idle 状态以后，才能把任务塞回队列里
    // add_task(task);
    let current_task_cx_ptr = task.get_task_cx_ptr() as *mut TaskContext;
    let idle_task_cx_ptr = cpu_local.get_idle_task_cx_ptr();
    //println!("idle task context ptr {:x}", idle_task_cx_ptr as usize);
    //drop(task_inner);
    drop(task);
    drop(cpu_local);
    // 切换回 run_tasks() 中
    unsafe {
        __switch(current_task_cx_ptr, idle_task_cx_ptr);
    }
}

/// 终止当前用户程序，回到 idle 状态
pub fn exit_current_task(exit_code: i32) {
    let cpu_id = get_cpu_id();
    let mut cpu_local = CPU_CONTEXTS[cpu_id].lock();
    let task = cpu_local.current().unwrap();
    // let task_inner = task.lock();
    task.set_status(TaskStatus::Dying);
    task.set_exit_code(exit_code);
    // clear_child_tid 的值不为 0，则将这个用户地址处的值写为0
    let addr = task.inner.lock().clear_child_tid;
    if addr != 0 {
        // 确认这个地址在用户地址空间中。如果没有也不需要报错，因为线程马上就退出了
        if task.vm.lock().manually_alloc_page(addr).is_ok() {
            info!("exit, clear tid {:x}", addr);
            unsafe {
                *(addr as *mut i32) = 0;
            }
        }
    }
    //println!("[cpu {}] tid {} exited with code {}", cpu_id, task.get_tid_num(), exit_code);
    let idle_task_cx_ptr = cpu_local.get_idle_task_cx_ptr();
    //println!("idle task context ptr {:x}", idle_task_cx_ptr as usize);
    //drop(task_inner);
    drop(task);
    drop(cpu_local);
    // 切换回 run_tasks() 中
    unsafe {
        __move_to_context(idle_task_cx_ptr);
    }
}

/// 通过 exec 系统调用，直接切换到新的用户进程
pub fn exec_new_task() {
    let cpu_id = get_cpu_id();
    let cpu_local = CPU_CONTEXTS[cpu_id].lock();
    let task = cpu_local.current().unwrap();
    //println!("user vm {:#x?}", task.inner.lock().vm);
    let current_task_cx_ptr = task.get_task_cx_ptr() as *mut TaskContext;
    drop(task);
    drop(cpu_local);
    unsafe {
        __move_to_context(current_task_cx_ptr);
    }
}
/// 处理退出的任务：
/// 将它的子进程全部交给初始进程 ORIGIN_USER_PROC，然后标记当前进程的状态为 Zombie。
/// 这里会需要获取当前核正在运行的用户程序、ORIGIN_USER_PROC、所有子进程的锁。
///
/// 这里每修改一个子进程的 parent 指针，都要重新用 try_lock 拿子进程的锁和 ORIGIN_USER_PROC 的锁。
///
/// 如果不用 try_lock ，则可能出现如下的死锁情况：
/// 1. 当前进程和子进程都在这个函数里
/// 2. 当前进程拿到了 ORIGIN_USER_PROC 的锁，而子进程在函数开头等待 ORIGIN_USER_PROC 的锁
/// 3. 当前进程尝试修改子进程的 parent，但无法修改。因为子进程一直拿着自己的锁，它只是在等 ORIGIN_USER_PROC
///
/// 使用 try_lock 之后，如果出现多个父子进程竞争锁的情况，那么：
/// 1. 如果拿到 ORIGIN_USER_PROC 的锁的进程的子进程都没有在竞争这个锁，那么它一定可以顺利拿到自己的所有子进程的锁，并正常执行完成。
/// 2. 否则，它会因为无法拿到自己的某个子进程的锁而暂时放弃 ORIGIN_USER_PROC 的锁。
///
/// 因为进程之间的 parent/children 关系是一棵树，所以在任意时刻一定会有上述第一种情况的进程存在。
/// 所以卡在这个函数上的进程最终一定能以某种顺序依次执行完成，也就消除了死锁。
///
fn handle_zombie_task(_cpu_local: &mut CpuLocal, task: Arc<TaskControlBlock>) {
    let mut tcb_inner = task.inner.lock();
    //let task_inner = task.lock();
    for child in tcb_inner.children.iter() {
        loop {
            // 这里把获取子进程的锁放在外层，是因为如果当前进程和子进程都在这个函数里，
            // 父进程可能拿到 start_proc 的锁，但一定拿不到 child 的锁。
            // 因为每个进程在进这个函数时都拿着自己的锁，所以此时只有子进程先执行完成，父进程才能继续执行。
            // 为了防止父进程反复抢 start_proc 的锁又不得不释放，所以把获取子进程的锁放在外层
            if let Some(mut child_inner) = child.inner.try_lock() {
                if tcb_inner.ppid == NO_PARENT || IS_TEST_ENV {
                    child_inner.ppid = NO_PARENT;
                    break;
                } else if let Some(mut start_proc_tcb_inner) =
                    ORIGIN_USER_PROC.clone().inner.try_lock()
                {
                    child_inner.parent = Some(Arc::downgrade(&ORIGIN_USER_PROC));
                    child_inner.ppid = 0;
                    start_proc_tcb_inner.children.push(child.clone());
                    // 拿到锁并修改完成后，退到外层循环去修改下一个子进程
                    break;
                }
            }
            // 只要没拿到任意一个锁，就继续循环
        }
    }
    tcb_inner.children.clear();
    tcb_inner.task_status = TaskStatus::Zombie;
    // 在测试环境中时，手动检查退出时的 exit_code
    if IS_TEST_ENV && task.pid == task.tid.0 {
        show_testcase_result(tcb_inner.exit_code);
    }
    //println!("tid {} is dead", task.tid.0);
    //println!("dead time {}", crate::timer::get_time());
    // 退出时向父进程发送信号，其中选项可被 sys_clone 控制
    if task.send_sigchld_when_exit || task.pid == task.tid.0 {
        send_signal(tcb_inner.ppid, SignalNo::SIGCHLD as usize);
    }
    // 通知全局表将 signals 删除
    global_logoff_signals(task.tid.0);
    // 释放用户段占用的物理页面
    // 如果这里不释放，等僵尸进程被回收时 MemorySet 被 Drop，也可以释放这些页面

    // <- 之前是那么考虑的，但内存压力大的情况下好像可能不够用，还是提前gc吧
    task.vm.lock().clear_user();
    
}

/// 处理用户程序的缺页异常
pub fn handle_user_page_fault(vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
    let cpu_id = get_cpu_id();
    let cpu_local = CPU_CONTEXTS[cpu_id].lock();
    if let Some(task) = cpu_local.current() {
        task.vm.lock().handle_page_fault(vaddr, access_flags)
    } else {
        Err(OSError::Task_NoTrapHandler)
    }
}

/// 获取当前核正在运行的进程的TCB。
/// 如果当前核没有任务，则返回 None
pub fn get_current_task() -> Option<Arc<TaskControlBlock>> {
    Some(CPU_CONTEXTS[get_cpu_id()].lock().current.as_ref()?.clone())
}

///从内核态进入用户态时统计时间
pub fn timer_kernel_to_user() {
    get_current_task().unwrap().time.lock().timer_kernel_to_user();
}

///从用户态进入内核态时统计时间
pub fn timer_user_to_kernel() {
    get_current_task().unwrap().time.lock().timer_user_to_kernel();
}

/// 处理当前线程的信号
pub fn handle_signals() {
    // 仅在 trap 时调用这个函数，所以保证当前线程和对应 signals 都是存在的
    let task = get_current_task().unwrap();
    // 如果其他线程正在向这里发送信号，则当前线程在此被阻塞
    let mut sig_inner = task.signal_receivers.lock();
    let handler = task.signal_handlers.lock();
    if let Some(signum) = sig_inner.get_one_signal() {
        let signal = SignalNo::from(signum);
        //println!("tid {} handling signal: {:#?}", task.get_tid_num(), signal);
        // 保存成功说明当前没有在处理其他信号
        if task.save_trap_cx_if_not_handling_signals() {
            // 如果有，则调取处理函数
            if let Some(action) = handler.get_action_ref(signum) {
                //println!("flags: {:#?}", action.flags);
                if action.handler == SIG_IGN {
                    return;
                }
                // 保存后开始操作准备修改上下文，跳转到用户的信号处理函数
                let trap_cx = unsafe { &mut *task.kernel_stack.get_first_context() };
                trap_cx.set_ra(action.get_restorer());
                //println!("restorer {}", action.get_restorer());
                // 这里假设了用户栈没有溢出
                info!("sp now {:x}", trap_cx.get_sp());
                let mut sp = trap_cx.get_sp() - USER_STACK_RED_ZONE;
                let old_pc = trap_cx.get_sepc();
                trap_cx.set_sepc(action.handler);
                trap_cx.set_a0(signum);
                if action.flags.contains(SigActionFlags::SA_SIGINFO) {
                    task.save_if_set_siginfo(true);
                    // 如果带 SIGINFO，则需要在用户栈上放额外的信息
                    sp = (sp - size_of::<SigInfo>()) & !0xf;
                    info!("add siginfo at {:x}", sp);
                    let mut info = SigInfo::default();
                    info.si_signo = signum as i32;
                    unsafe {
                        *(sp as *mut SigInfo) = info;
                    }
                    trap_cx.set_a1(sp);
                    sp = (sp - size_of::<SignalUserContext>()) & !0xf;
                    unsafe {
                        *(sp as *mut SignalUserContext) =
                            SignalUserContext::init(sig_inner.mask.0 as u64, old_pc);
                    }
                    trap_cx.set_a2(sp);
                    //let v = unsafe { *((sp + 0xb0) as *const usize) };
                    //info!("read {} pc {}", v, old_pc);

                    let tp = trap_cx.x[4];
                    //let cancel = tp - 156;
                    info!("val {}", unsafe { *((tp - 168) as *const u32) }); //tid
                    info!("val {}", unsafe { *((tp - 156) as *const u32) });
                    info!("val {}", unsafe { *((tp - 152) as *const u8) });
                    info!("val {}", unsafe { *((tp - 151) as *const u8) });
                }
                trap_cx.set_sp(sp);
                //info!("into signal handler, sp = {:x} old_pc = {:x}", sp, old_pc);
            } else {
                // 否则，查找默认处理方式
                match SigActionDefault::of_signal(signal) {
                    SigActionDefault::Terminate => {
                        // 这里不需要 drop(task)，因为当前函数没有用到 task_inner，在 task.save_trap... 内部用过后已经 drop 了
                        drop(handler);
                        drop(sig_inner);
                        exit_current_task(0);
                    }
                    SigActionDefault::Ignore => {
                        // 忽略信号时，要将已保存的上下文删除
                        task.load_trap_cx_if_handling_signals();
                    }
                }
            }
        } else if signal == SignalNo::SIGSEGV || signal == SignalNo::SIGBUS {
            
            //在处理信号的过程中又触发 SIGSEGV 或 SIGBUS，那么说明该直接结束了，否则会无限递归触发
            exit_current_task(-1);
        }
    }
    //info!("signal handler finish");
}

/// 从信号处理中返回。
/// 为了适配 syscall，返回原来的用户上下文中的 a0 的值
pub fn signal_return() -> isize {
    // 仅在 sys_sigreturn 中调用这个函数，所以保证当前线程和对应 signals 都是存在的
    let task = get_current_task().unwrap();
    //let signals = get_signals_from_tid(task.tid.0).unwrap();
    if task.load_trap_cx_if_handling_signals() {
        // 上面已经 load 了，此处获取的值是原来的上下文
        let trap_cx = unsafe { &mut *task.kernel_stack.get_first_context() };
        trap_cx.get_a0() as isize
    } else {
        // 如果当前没有在信号处理函数中，却调用了 sigreturn，则返回 -1
        -1
    }
}
