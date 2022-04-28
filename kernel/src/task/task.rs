//! 用户程序的数据及状态信息
//! 一个 TaskControlBlock 包含了一个任务(之后会是进程)的所有信息

#![deny(missing_docs)]

use super::{TaskContext, MemorySet};

/// 任务控制块，包含一个用户程序的所有状态信息，但不包括与调度有关的信息
// 默认在TCB的外层有 Arc<Mutex<>>，所以内部的结构没有用锁保护
pub struct TaskControlBlock {
    /// 任务执行状态
    pub task_status: TaskStatus,
    /// 上下文信息，用于切换，包含所有必要的寄存器
    /// 实际在第一次初始化时还包含了用户程序的入口地址和用户栈
    pub task_cx: TaskContext,
    /// 任务的内存段(内含页表)，同时包括用户态和内核态
    pub vm: MemorySet,
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
    /// 已执行完成
    Exited, 
}
