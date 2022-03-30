const SBI_CONSOLE_PUTCHAR: usize = 1;

#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        core::arch::asm!("ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which,
            lateout("a0") ret,
        );
    }
    ret
}

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

pub fn console_putint(val: usize) {
    console_putchar(b'0' as usize + val);
}

pub fn print(s: &str) {
    for c in s.bytes() {
        console_putchar(c as usize);
    }
}
