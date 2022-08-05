//! 关于切换执行流的汇编 `__switch`
//!
//! 通过内核栈，将当前 cpu 的寄存器切换到另一个任务的寄存器，包括 ra 和 sp
//! 也就是说调用 __switch 的位置和从 __switch 返回的地址可能不同

use super::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

extern "C" {
    /// Switch to the context of `next_task_cx_ptr`, saving the current context
    /// in `current_task_cx_ptr`.
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
    pub fn __move_to_context(next_task_cx_ptr: *const TaskContext);
}
