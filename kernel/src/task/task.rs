//! 用户程序的数据及状态信息
//! 一个 TaskControlBlock 包含了一个任务(或进程)的所有信息

#![deny(missing_docs)]

use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use alloc::string::String;
use lock::Mutex;
use core::slice::Iter;

use crate::loader::{get_app_data, get_app_data_by_name};
use crate::loaders::ElfLoader;
use crate::memory::{MemorySet, Pid, new_memory_set_for_task};
use crate::trap::TrapContext;
use crate::arch::get_cpu_id;

use super::{TaskContext, KernelStack};

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
    /// 进程 id
    pub pid: Pid,
    /// 任务的状态信息
    pub inner: Mutex<TaskControlBlockInner>,
}

/// 任务控制块的可变部分
pub struct TaskControlBlockInner {
    /// 任务执行状态
    pub task_status: TaskStatus,
    /// 上下文信息，用于切换，包含所有必要的寄存器
    /// 实际在第一次初始化时还包含了用户程序的入口地址和用户栈
    pub task_cx: TaskContext,
    /// 任务的内存段(内含页表)，同时包括用户态和内核态
    pub vm: MemorySet,
    // 父进程
    pub parent: Option<Weak<TaskControlBlock>>,
    // 子进程
    pub children: Vec<Arc<TaskControlBlock>>,
    // sys_exit 时输出的值
    pub exit_code: i32,
}

impl TaskControlBlock {
    /// 从用户程序名生成 TCB
    pub fn from_app_name(app_name: &str) -> Option<Self> {
        if let Some(raw_data) = get_app_data_by_name(app_name) {
            Some(Self::from_elf(raw_data))
        } else {
            None
        }
    }
    /// 从 app_id 生成 TCB
    pub fn from_app_id(app_id: usize) -> Self {
        // 获取用户库编译链接好的(elf 格式的)用户数据
        let raw_data = get_app_data(app_id);
        Self::from_elf(raw_data)
    }
    /// 从ELF文件初始化一个TCB。
    /// 目前的实现返回 Self ，也就是说如果这个过程不成功，则直接 panic
    pub fn from_elf(raw_data: &[u8]) -> Self {
        // 新建页表，包含内核段
        let mut vm = new_memory_set_for_task().unwrap();
        // 然后将用户地址段信息插入页表和 VmArea
        let loader = ElfLoader::new(raw_data).unwrap();
        let args = vec![String::from(".")];
        let (user_entry, user_stack) = loader.init_vm(&mut vm, args).unwrap();
        //println!("user MemorySet {:#x?}", vm);
        // 初始化内核栈，它包含关于进入用户程序的所有信息
        let kernel_stack = KernelStack::new().unwrap();
        //kernel_stack.print_info();
        let pid = Pid::new().unwrap();
        // println!("pid = {}", pid.0);
        let stack_top = kernel_stack.push_first_context(TrapContext::app_init_context(user_entry, user_stack));
        TaskControlBlock {
            kernel_stack: kernel_stack,
            pid: pid,
            inner: Mutex::new(TaskControlBlockInner {
                task_cx: TaskContext::goto_restore(stack_top),
                task_status: TaskStatus::Ready,
                vm: vm,
                parent: None,
                children: Vec::new(),
                exit_code: 0
            }),
        }
    }
    /// 从 fork 系统调用初始化一个TCB，并设置子进程对用户程序的返回值为0。
    /// 
    /// 这里只把父进程内核栈栈底的第一个 TrapContext 复制到子进程，
    /// 所以**必须保证对这个函数的调用是来自用户异常中断，而不是内核异常中断**。因为只有这时内核栈才只有一层 TrapContext。
    pub fn from_fork(self: &Arc<TaskControlBlock>) -> Arc<Self> {
        let mut inner = self.inner.lock();
        // 与 new 方法不同，这里从父进程的 MemorySet 生成子进程的
        let mut vm = inner.vm.copy_as_fork().unwrap(); 
        let kernel_stack = KernelStack::new().unwrap();
        // 与 new 方法不同，这里从父进程的 TrapContext 复制给子进程
        let mut trap_context = TrapContext::new();
        unsafe { trap_context = *self.kernel_stack.get_first_context(); }
        // 手动设置返回值为0，这样两个进程返回用户时除了返回值以外，都是完全相同的
        trap_context.set_a0(0);
        let stack_top = kernel_stack.push_first_context(trap_context);
        let pid = Pid::new().unwrap();

        let new_tcb = Arc::new(TaskControlBlock {
            pid: pid,
            kernel_stack: kernel_stack,
            inner: {
                Mutex::new(TaskControlBlockInner {
                    task_cx: TaskContext::goto_restore(stack_top),
                    task_status: TaskStatus::Ready,
                    vm: vm,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                })
            },
        });
        inner.children.push(new_tcb.clone());
        new_tcb
    }
    /// 从 exec 系统调用修改当前TCB：
    /// 1. 从 ELF 文件中生成新的 MemorySet 替代当前的
    /// 2. 修改内核栈栈底的第一个 TrapContext 为新的用户程序的入口
    pub fn exec(&self, raw_data: &[u8]) {
        let mut inner = self.inner.lock();
        // 清空 MemorySet 中用户段的地址
        inner.vm.clear_user();
        // 然后把新的信息插入页表和 VmArea
        let loader = ElfLoader::new(raw_data).unwrap();
        let args = vec![String::from(".")];
        let (user_entry, user_stack) = loader.init_vm(&mut inner.vm, args).unwrap();
        // 此处实际上覆盖了 kernel_stack 中原有的 TrapContext，内部用 unsafe 规避了此处原本应有的 mut
        self.kernel_stack.push_first_context(TrapContext::app_init_context(user_entry, user_stack));
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
    /// 获取 pid 的值，不会转移或释放 Pid 的所有权
    pub fn get_pid_num(&self) -> usize {
        self.pid.0
    }
    /// 如果当前进程已是运行结束，则获取其 exit_code，否则返回 None
    pub fn get_code_if_exit(&self) -> Option<i32> {
        let inner = self.inner.try_lock()?; 
        match inner.task_status {
            TaskStatus::Zombie => Some(inner.exit_code),
            _ => None
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
