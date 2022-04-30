//! 保存/恢复一个用户栈所必要的信息

/// 一个任务的上下文信息，包含所有必要的寄存器
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
    /// return address ( e.g. __restore ) of __switch ASM function
    ra: usize,
    /// kernel stack pointer of app
    sp: usize,
    /// callee saved registers:  s 0..11
    s: [usize; 12],
}

impl TaskContext {
    /// 初始化一个 TaskContext，其中所有值为 0
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// 初始化一个 TaskContext，其中 ra 为 __restore() 地址， sp 为输入的内核栈地址，
    /// 其余所有值为 0
    pub fn goto_restore(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }

    /// 获取Context中 ra 寄存器的值
    pub fn get_ra(&self) -> usize {
        self.ra
    }
}
