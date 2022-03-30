use core::fmt::{Arguments, Result, Write};

use lock::mutex::Mutex;

struct Stdout;

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

pub fn stdout_puts(fmt: Arguments) {
    static STDOUT: Mutex<Stdout> = Mutex::new(Stdout);
    STDOUT.lock().write_fmt(fmt).unwrap();
}
