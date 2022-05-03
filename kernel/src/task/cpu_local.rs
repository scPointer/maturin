//! 每个核当前正在运行的任务及上下文信息

#![deny(missing_docs)]

use alloc::vec::Vec;
use alloc::sync::Arc;
use lock::Mutex;
use lazy_static::*;

use crate::constants::CPU_NUM;
use crate::error::OSError;
use crate::trap::TrapContext;
use crate::arch::get_cpu_id;

use super::__switch;
use super::{fetch_task_from_scheduler, push_task_to_scheduler};
use super::{TaskContext, TaskControlBlock, TaskStatus};

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
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }

    pub fn run_tasks(&mut self) -> ! {
        loop {
            if let Some(task) = fetch_task_from_scheduler() {
                let idle_task_cx_ptr = self.get_idle_task_cx_ptr();
                //let mut task_inner = task.inner.lock();
                let next_task_cx_ptr = task.get_task_cx_ptr();
                // 因为 task 此时已从调度器取出，所以只有当前核可以访问它
                // 所以不需要在修改过程中一直持有 inner 锁然后手动 drop
                task.set_status(TaskStatus::Running);
                //drop(task_inner);
                self.current = Some(task);
                // 切换到用户程序执行
                unsafe {
                    __switch(idle_task_cx_ptr, next_task_cx_ptr);
                }
                // 在上面的用户程序中，会执行 suspend_current_and_run_next() 或  exit_current_and_run_next(exit_code: i32)
                // 在其中会修改 current.task_status 和 exit_code，但任务本身还在被当前 CPU 占用，需要下面再将其插入队列或

                // 此时已切回空闲任务
                if let Some(task) = self.take_current() {
                    match task.get_status() {
                        TaskStatus::Ready => {

                        }
                        TaskStatus::Zombie => {

                        }
                        _ => {
                            panic!("invalid task status when switched out");
                        }
                    }
                } else {
                    panic!("[cpu {}] CpuLocal: switched from empty task", get_cpu_id());
                }
            }
        }
    }
}

lazy_static! {
    /// 所有 CPU 的上下文信息
    pub static ref CPU_CONTEXTS: Vec<Mutex<CpuLocal>> = unsafe {
        let mut cpu_contexts: Vec<Mutex<CpuLocal>> = Vec::new();
        for i in 0..CPU_NUM {
            cpu_contexts.push(Mutex::new(CpuLocal::new()));
        }
        cpu_contexts
    };
}

/// 开始执行用户程序
pub fn run_user_tasks() -> ! {
    let mut cpu_local = CPU_CONTEXTS[get_cpu_id()].lock();
    cpu_local.run_tasks()
}

