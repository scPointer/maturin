//! 标准输入
//!
//! 目前只能每次输入一个 u8，所以看起来封装会比较过度

/// 标准输入。
pub struct Stdin;

impl Stdin {
    /// 从输入流读取一个字符
    #[inline]
    #[allow(deprecated)]
    pub fn getchar(&self) -> u8 {
        sbi_rt::legacy::console_getchar() as _
    }
}

pub static STDIN: lock::Mutex<Stdin> = lock::Mutex::new(Stdin);

/// 从输入流读取一个字符
pub fn getchar() -> u8 {
    STDIN.lock().getchar()
}
