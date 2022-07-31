//! 标准输入
//! 
//! 目前只能每次输入一个 u8，所以看起来封装会比较过度

//#![deny(missing_docs)]

use lock::Mutex;
use lazy_static::*;

/// 绕过所有锁读取一个字符
fn getchar_raw() -> u8 {
    super::sbi::console_getchar() as u8
}

/// 标准输入。
pub struct Stdin;

impl Stdin {
    /// 从输入流读取一个字符
    pub fn getchar(&self) -> u8 {
        getchar_raw()
    }
}

lazy_static::lazy_static! {
    pub static ref STDIN: Mutex<Stdin> = Mutex::new(Stdin);
}

/// 从输入流读取一个字符
pub fn getchar() -> u8 {
    STDIN.lock().getchar()
}

