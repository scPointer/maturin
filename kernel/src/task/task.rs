//! Types related to task management

use super::TaskContext;
use super::{Arc, Mutex, MemorySet};

//#[derive(Copy, Clone)]
// 默认在TCB的外层有 Arc<Mutex<>>，所以内部的结构没有用锁保护
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub vm: MemorySet,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
