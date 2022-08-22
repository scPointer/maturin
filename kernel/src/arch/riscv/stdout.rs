use core::fmt::{Arguments, Result, Write};
use lock::Mutex;

/// 绕过所有锁打印一个字符
#[inline]
fn putchar_raw(c: u8) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c as _);
}

/// 标准输出
pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            if c == 127 {
                putchar_raw(8);
                putchar_raw(b' ');
                putchar_raw(8);
            } else {
                putchar_raw(c);
            }
        }
        Ok(())
    }
}

pub static STDOUT: Mutex<Stdout> = Mutex::new(Stdout);
pub static STDERR: Mutex<Stdout> = Mutex::new(Stdout);

/// 输出到 stdout
#[inline]
pub fn stdout_puts(fmt: Arguments) {
    STDOUT.lock().write_fmt(fmt).unwrap();
}

/// 输出到 stderr
#[inline]
pub fn stderr_puts(fmt: Arguments) {
    // 使 stdout 不要干扰 stderr 输出
    // 如果能拿到锁，说明此时没有核在输出 STDOUT，那么 STDERR 优先输出，不让其他核打断
    // 如不能，则有可能 STDOUT 已卡死了，此时也直接输出
    let _stdout = STDOUT.try_lock();
    STDERR.lock().write_fmt(fmt).unwrap();
}
