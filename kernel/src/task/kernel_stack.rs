//! 内核栈
//! 内部保存了 Frame，所以栈本身占用的内存会在结构被 Drop 时释放掉

//#![deny(missing_docs)]

use crate::constants::{KERNEL_STACK_SIZE, PAGE_SIZE};
use crate::error::{OSError, OSResult};
use crate::memory::Frame;
use crate::trap::TrapContext;

/// 内核栈，会通过帧分配器申请一段内存
/// 在内核态时，这段内存是在 physical memory 上的，因此可以直接访问
/// 这省去了修改 MemorySet 和页表的步骤，比较快，但也意味着没有 shadow page，需要由其他机制实现(Todo: trap.S 中判断)
/// 因为 physical memory 中的所有页都有 READ/WRITE 权限
pub struct KernelStack {
    frame: Frame,
}

impl KernelStack {
    /// 创建内核栈并申请内存
    pub fn new() -> OSResult<Self> {
        // if let Some(frame) = Frame::new_contiguous(KERNEL_STACK_SIZE / PAGE_SIZE, 9) {
        if let Some(frame) = Frame::new_contiguous(KERNEL_STACK_SIZE / PAGE_SIZE, 0) {
            Ok(KernelStack { frame: frame })
        } else {
            Err(OSError::Task_RunOutOfMemory)
        }
    }
    /// 获取栈底，也即刚初始化时的栈顶
    fn get_sp(&self) -> usize {
        self.frame.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    /// 在空栈里压栈一个 TrapContext。
    /// 之后如果发生内核异常中断，则 trap.S 会进行压栈，使得栈里有更多个 TrapContext。
    pub fn push_first_context(&self, trap_cx: TrapContext) -> usize {
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        trap_cx_ptr as usize
    }
    /// 获取第一个 TrapContext 的地址
    pub fn get_first_context(&self) -> *mut TrapContext {
        (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext
    }
    /// 打印栈所占用的内存地址
    pub fn print_info(&self) {
        println!(
            "kernel stack at: [{:x}, {:x}]",
            self.get_sp() - KERNEL_STACK_SIZE,
            self.get_sp()
        );
    }
}
