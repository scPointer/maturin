//! 标准输入
//!
//! 目前只能每次输入一个 u8，所以看起来封装会比较过度

use polyhal::debug::DebugConsole;

/// 标准输入。
pub struct Stdin;

impl Stdin {
    /// 从输入流读取一个字符
    #[inline]
    #[allow(deprecated)]
    pub fn getchar(&self) -> u8 {
        match DebugConsole::getchar() {
            Some(c) => c,
            None => 0,
        }
    }
}

pub static STDIN: lock::Mutex<Stdin> = lock::Mutex::new(Stdin);

/// 从输入流读取一个字符
pub fn getchar() -> u8 {
    STDIN.lock().getchar()
}
