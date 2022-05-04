//! 任务管理
//!
//! 全局常量 TASK_MANAGER 在初始化过程中导入所有用户程序的数据，并分别保存在一个 TaskControlBlock 中
//! 所有的 TaskControlBlock 都放在 Arc<Mutex<TaskManagerInner>> 中，
//! 每个核需要切换任务时都需要拿到这个锁，且从调度开始到结束**必须一直持有**这个锁
//!

#![deny(missing_docs)]

use lazy_static::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{Ordering, AtomicUsize};
use lock::Mutex;

mod context;
mod switch;
mod kernel_stack;
mod scheduler;
mod cpu_local;


#[allow(clippy::module_inception)]
mod task;

use crate::arch::get_cpu_id;
use crate::constants::ORIGIN_USER_PROC_NAME;

use switch::{__switch, __move_to_context};
pub use task::{TaskControlBlock, TaskStatus};
pub use context::TaskContext;
pub use kernel_stack::KernelStack;
pub use scheduler::{push_task_to_scheduler, fetch_task_from_scheduler};
pub use cpu_local::{
    suspend_current_task,
    exit_current_task,
    handle_user_page_fault,
    run_tasks,
    get_current_task,
    exec_new_task,
};

lazy_static! {
    /// 第一个用户程序
    /// 任务调度器启动时会自动在队列中插入它作为第一个用户程序
    pub static ref ORIGIN_USER_PROC: Arc<TaskControlBlock> = Arc::new(
        TaskControlBlock::from_app_name(ORIGIN_USER_PROC_NAME).unwrap()
    );
}
