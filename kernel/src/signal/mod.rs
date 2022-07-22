//! 信号模块，用于 sigaction / sigreturn / kill 等 syscall
//! 信号模块和 task 管理的进程/线程相关，但又相对独立；
//! 且如果单纯作为线程的一部分，容易因为信号发送的任意性导致死锁，因此单独列出来。
//! 
//! 目前的模型中，不采用 ipi 实时发送信号，而是由被目标线程在 trap 时处理。因此需要开启**时钟中断**来保证信号能实际送到

mod signal_no;
pub use signal_no::SignalNo;
mod sig_action;
pub use sig_action::{SigAction, SigActionFlags};
mod bitset;
pub use bitset::Bitset;

use crate::constants::SIGSET_SIZE_IN_BIT;

/// 一个进程对应的信号相关变量及处理
#[derive(Clone, Copy)]
pub struct Signals {
    /// 所有的处理函数
    pub actions: [Option<SigAction>; SIGSET_SIZE_IN_BIT],
    /// 掩码，表示哪些信号是当前线程不处理的。（目前放在进程中，实现了线程之后每个线程应该各自有一个）
    pub mask: Bitset,
    /// 当前已受到的信号
    pub sig_received: Bitset,
}

impl Signals {
    /// 新建一个信号模块
    pub fn new() -> Self {
        Self {
            actions: [None; SIGSET_SIZE_IN_BIT],
            mask: Bitset::new(0),
            sig_received: Bitset::new(0),
        }
    }
    /// 清空模块。
    /// exec时需要将信号模块恢复为默认。但因为其他核可能在往当前线程写信号，所以这里手动清空而不是重新 new 一个
    pub fn clear(&mut self) {
        for action in &mut self.actions {
            action.take();
        }
        self.mask = Bitset::new(0);
        self.sig_received = Bitset::new(0);
    }
}