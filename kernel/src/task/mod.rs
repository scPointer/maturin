//! 任务管理
//!
//! 全局常量 TASK_MANAGER 在初始化过程中导入所有用户程序的数据，并分别保存在一个 TaskControlBlock 中
//! 所有的 TaskControlBlock 都放在 Arc<Mutex<TaskManagerInner>> 中，
//! 每个核需要切换任务时都需要拿到这个锁，且从调度开始到结束**必须一直持有**这个锁

mod clone_flags;
mod context;
mod cpu_local;
mod kernel_stack;
mod scheduler;
mod switch;
mod task;
mod time_stat;

use crate::constants::{ORIGIN_USER_PROC_NAME, ROOT_DIR};
use alloc::sync::Arc;
use switch::{__move_to_context, __switch};

pub use clone_flags::CloneFlags;
pub use context::TaskContext;
pub use cpu_local::{
    exec_new_task, exit_current_task, get_current_task, handle_signals, handle_user_page_fault,
    run_tasks, signal_return, suspend_current_task, timer_kernel_to_user,
    timer_user_to_kernel,
};
pub use kernel_stack::KernelStack;
pub use scheduler::Scheduler;
pub use scheduler::{fetch_task_from_scheduler, push_task_to_scheduler};
pub use task::{TaskControlBlock, TaskControlBlockInner, TaskStatus};
pub use time_stat::{TimeStat, ITimerVal};

lazy_static::lazy_static! {
    /// 第一个用户程序
    /// 任务调度器启动时会自动在队列中插入它作为第一个用户程序
    pub static ref ORIGIN_USER_PROC: Arc<TaskControlBlock> = Arc::new(
        TaskControlBlock::from_app_name(ROOT_DIR, 0, vec![ORIGIN_USER_PROC_NAME.into()]).unwrap()
    );
}
