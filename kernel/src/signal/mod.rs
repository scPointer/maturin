//! 信号模块，用于 sigaction / sigreturn / kill 等 syscall
//! 信号模块和 task 管理的进程/线程相关，但又相对独立；
//! 且如果单纯作为线程的一部分，容易因为信号发送的任意性导致死锁，因此单独列出来。
//!
//! 目前的模型中，不采用 ipi 实时发送信号，而是由被目标线程在 trap 时处理。因此需要开启**时钟中断**来保证信号能实际送到

mod signal_no;
pub use signal_no::SignalNo;
mod sig_action;
pub use sig_action::{SigAction, SigActionDefault, SigActionFlags, SIG_DFL, SIG_IGN};
mod sig_info;
pub use sig_info::SigInfo;
mod ucontext;
pub use ucontext::SignalUserContext;
mod bitset;
pub use bitset::Bitset;
mod long_bitset;
pub use long_bitset::LongBitset;
mod shadow_bitset;
pub use shadow_bitset::ShadowBitset;
mod tid2signals;
use crate::constants::SIGSET_SIZE_IN_BIT;
pub use tid2signals::{get_signals_from_tid, global_logoff_signals, global_register_signals};

/// 处理信号的结构，每个线程有一个，根据 clone 的参数有可能是共享的
#[derive(Clone, Copy)]
pub struct SignalHandlers {
    /// 所有的处理函数
    actions: [Option<SigAction>; SIGSET_SIZE_IN_BIT],
}

impl SignalHandlers {
    /// 新建一个信号模块
    pub fn new() -> Self {
        Self {
            actions: [None; SIGSET_SIZE_IN_BIT],
        }
    }
    /// 清空模块。
    /// exec时需要将信号模块恢复为默认。
    pub fn clear(&mut self) {
        for action in &mut self.actions {
            action.take();
        }
    }
    /// 获取某个信号对应的 SigAction。
    /// 因为 signum 的范围是 \[1,64\]，所以要 -1
    pub fn get_action<'a>(&self, signum: usize, action_pos: *mut SigAction) {
        if let Some(action) = self.actions[signum - 1] {
            unsafe {
                *action_pos = action;
            }
        }
    }
    /// 获取某个信号对应的 SigAction，如果存在，则返回其引用
    /// 因为 signum 的范围是 \[1,64\]，所以要 -1
    pub fn get_action_ref<'a>(&self, signum: usize) -> &Option<SigAction> {
        if self.actions[signum - 1].is_some() && self.actions[signum - 1].unwrap().handler == SIG_DFL {
            &None
        } else {
            &self.actions[signum - 1]
        }
        //if signum != 33 {&self.actions[signum - 1]} else {&None}
    }
    /// 修改某个信号对应的 SigAction。
    /// 因为 signum 的范围是 \[1,64\]，所以内部要 -1
    pub fn set_action(&mut self, signum: usize, action_pos: *const SigAction) {
        unsafe {
            self.actions[signum - 1] = Some(*action_pos);
            //self.actions[signum - 1].as_mut().unwrap().flags |= SigActionFlags::SA_SIGINFO;
        }
    }
}

/// 接受信号的结构，每个线程都独有，不会共享
#[derive(Clone, Copy)]
pub struct SignalReceivers {
    /// 掩码，表示哪些信号是当前线程不处理的。（目前放在进程中，实现了线程之后每个线程应该各自有一个）
    pub mask: Bitset,
    /// 当前已受到的信号
    pub sig_received: Bitset,
}

impl SignalReceivers {
    /// 新建一个处理模块
    pub fn new() -> Self {
        Self {
            mask: Bitset::new(0),
            sig_received: Bitset::new(0),
        }
    }
    /// 清空模块。
    pub fn clear(&mut self) {
        self.mask = Bitset::new(0);
        self.sig_received = Bitset::new(0);
    }
    /// 处理一个信号。如果有收到的信号，则返回信号编号。否则返回 None
    pub fn get_one_signal(&mut self) -> Option<usize> {
        self.sig_received.find_first_one(self.mask).map(|pos| {
            self.sig_received.remove_bit(pos);
            pos + 1
        })
    }

    /// 尝试添加一个 bit 作为信号。发送的信号如果在 mask 中，则仍然会发送，只是可能不触发
    /// 因为 signum 的范围是 \[1,64\]，所以内部要 -1
    ///
    /// 因为没有要求判断信号是否发送成功的要求，所有这里不设返回值
    pub fn try_add_bit(&mut self, signum: usize) {
        //info!("try add {}, mask = {:x}", signum, self.mask.0);
        self.sig_received.add_bit(signum - 1);
    }
}

/// 发送一个信号给进程 tid
pub fn send_signal(tid: usize, signum: usize) {
    if let Some(signals) = get_signals_from_tid(tid as usize) {
        // 获取目标线程(可以是自己)的 signals 数组
        signals.lock().try_add_bit(signum);
    }
}
