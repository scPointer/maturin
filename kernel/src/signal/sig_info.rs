//! 触发信号时的信息。当 SigAction 指定需要信息时，需要将其返回给用户

/// 错误信息
///
/// 详细定义见 `https://man7.org/linux/man-pages/man2/rt_sigaction.2.html`
/// 更准确的错误信息的内容比现在实现的要多很多，但剩下的部分根据信号不同，定义也会变得非常复杂
pub struct SigInfo {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
}

impl Default for SigInfo {
    fn default() -> Self {
        Self {
            si_signo: 0,
            si_errno: 0,
            si_code: -6, // -6 代表原因是 TKILL
        }
    }
}
