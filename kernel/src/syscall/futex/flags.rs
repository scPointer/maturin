//! futex 用到的参数定义
//!
//! 详见 `https://man7.org/linux/man-pages/man2/futex.2.html`

/// 对 futex 的操作
pub enum Flags {
    /// 检查用户地址 uaddr 处的值。如果不是要求的值则等待 wake
    WAIT = 0,
    /// 唤醒最多 val 个在等待 uaddr 位置的线程。
    WAKE = 1,
    /// 唤醒最多 val 个在等待 uaddr 位置的线程。如果有更多，则将它们转移到 uaddr2 处，至多转移 val2 个
    REQUEUE = 3,
    UNSUPPORTED,
}

/// 传入的选项
pub struct FutexFlag(i32);

impl FutexFlag {
    /// 生成选项
    pub fn new(val: i32) -> Self {
        Self(val)
    }
    /// 是否是当前地址空间内的。目前不支持跨进程的 futex
    pub fn is_private(&self) -> bool {
        (self.0 & 0x80) > 0
    }
    /// 选项对应的操作
    pub fn operation(&self) -> Flags {
        match self.0 & 0x7f {
            0 => Flags::WAIT,
            1 => Flags::WAKE,
            3 => Flags::REQUEUE,
            _ => Flags::UNSUPPORTED,
        }
    }
}
