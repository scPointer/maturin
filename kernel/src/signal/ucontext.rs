//! 触发信号时的额外用户信息。当 SigAction 指定需要信息时，需要将其返回给用户
//!
//! 这个文件的内容修改自 zCore (`https://github.com/rcore-os/zCore/`)

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SignalUserContext {
    pub flags: usize,
    pub link: usize,
    pub stack: SignalStack,
    pub sig_mask: u64,
    pub context: MachineContext,
}

impl SignalUserContext {
    pub fn init(mask: u64, pc: usize) -> Self {
        Self {
            flags: 0,
            link: 0,
            stack: SignalStack::default(),
            sig_mask: mask,
            context: MachineContext::init_with_pc(pc),
        }
    }
    /// pthread_cancel 会用到
    pub fn get_pc(&self) -> usize {
        self.context.pc
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: u32,
    pub size: usize,
}

impl Default for SignalStack {
    fn default() -> Self {
        // default to disabled
        SignalStack {
            sp: 0,
            flags: 2, // 选项 DISABLE,表示不使用栈
            size: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct MachineContext {
    pub reserved_: [usize; 16],
    // 目前只设置了 pc 值
    pub pc: usize,
    pub reserved: [usize; 17],
    pub fpstate: [usize; 66],
}

impl Default for MachineContext {
    fn default() -> Self {
        Self {
            reserved_: [0; 16],
            pc: 0,
            reserved: [0; 17],
            fpstate: [0; 66],
        }
    }
}

impl MachineContext {
    pub fn init_with_pc(pc: usize) -> Self {
        Self {
            reserved_: [0; 16],
            pc: pc,
            reserved: [0; 17],
            fpstate: [0; 66],
        }
    }
}
