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


#[allow(clippy::module_inception)]
mod task;

use crate::arch::get_cpu_id;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use kernel_stack::KernelStack;
pub use scheduler::*;
