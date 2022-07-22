//! 信号处理函数
//! 

use bitflags::*;
use super::Bitset;

/// SigAction::handler 的特殊取值，表示默认处理函数
pub const SIG_DFL: usize = 0; 
/// SigAction::handler 的特殊取值，表示忽略这个信号
pub const SIG_IGN: usize = 1;

/// 和信号处理函数相关的信息定义
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SigAction {
    /// 用户定义的处理函数地址
    /// 
    /// 1. 如果是上述特殊值 SIG_DFL 或 SIG_IGN，则按描述处理
    /// 2. 如果 flags 里没有 SA_SIGINFO，则它是 void (*sa_handler)(int);
    /// 3. 如果有，则它是 void (*sa_sigaction)(int, siginfo_t *, void *);
    /// 
    /// - 第一个参数 int 都是 sig_no 即信号编号。
    /// - 第二个参数 siginfo_t 是  {int si_signo; int si_errno; int si_code; ...}，总长为 128 Bytes
    /// - - 后边省略的参数根据信号不同有不同的定义，先不处理。
    /// - - si_signo 和前面的第一个参数相同
    /// - - si_errno 在 Linux 中不用
    /// - - si_code 一般表达出现信号的原因，但很复杂，下面仅处理在 glibc 中的常用定义
    pub handler: usize,
    /// 处理时指定的参数
    pub flags: SigActionFlags,
    /// 信号处理时的栈，也被视为 `signal trampoline`，由用户给出
    /// 
    /// 一般来说，需要 flags 里给出 SA_RESTORER 这里才有意义，但这里默认都是这种情况
    pub restorer: usize,
    /// 信号的掩码
    pub mask: Bitset,
}

/// bit 位参数
bitflags! {
    #[derive(Default)]
    /// 信号处理指定参数，详见 `https://man7.org/linux/man-pages/man2/rt_sigaction.2.html`
    pub struct SigActionFlags : usize {
        const SA_NOCLDSTOP = 1;
        const SA_NOCLDWAIT = 2;
        const SA_SIGINFO = 4;
        const SA_ONSTACK = 0x08000000;
        const SA_RESTART = 0x10000000;
        const SA_NODEFER = 0x40000000;
        const SA_RESETHAND = 0x80000000;
        const SA_RESTORER = 0x04000000;
    }
}
