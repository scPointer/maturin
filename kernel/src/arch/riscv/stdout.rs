use core::fmt::{Arguments, Result, Write};

use lock::Mutex;
use lazy_static::*;

pub struct Stdout;

fn putchar(c: u8) {
    super::sbi::console_putchar(c as usize);
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            if c == 127 {
                putchar(8);
                putchar(b' ');
                putchar(8);
            } else {
                putchar(c);
            }
        }
        Ok(())
    }
}

lazy_static::lazy_static! {
    pub static ref STDOUT: Mutex<Stdout> = Mutex::new(Stdout);
    pub static ref STDERR: Mutex<Stdout> = Mutex::new(Stdout);
}
pub fn stdout_puts(fmt: Arguments) {
    STDOUT.lock().write_fmt(fmt).unwrap();
}

pub fn stderr_puts(fmt: Arguments) {
    STDERR.lock().write_fmt(fmt).unwrap();
}
