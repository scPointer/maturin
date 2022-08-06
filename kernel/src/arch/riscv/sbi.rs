#![allow(unused)]

use sbi_rt::*;

#[inline]
pub fn set_timer(stime_value: u64) {
    sbi_rt::set_timer(stime_value);
}

#[inline]
pub fn shutdown() -> ! {
    sbi_rt::system_reset(RESET_TYPE_SHUTDOWN, RESET_REASON_NO_REASON);
    unreachable!()
}

#[inline]
pub fn console_getchar() -> usize {
    #[allow(deprecated)]
    sbi_rt::legacy::console_getchar()
}

#[inline]
pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    sbi_rt::legacy::console_putchar(c);
}

pub fn console_put_usize_in_hex(val: usize) {
    for x in 0..16 {
        let bit4 = ((val >> ((15 - x) * 4)) & 0xf) as u8;
        let c = if bit4 < 10 {
            b'0' + bit4
        } else {
            b'A' + bit4 - 10
        };
        console_putchar(c as usize);
    }
    for _ in 0..10 {
        console_putchar(b'-' as usize);
    }
    console_putchar(b'\n' as usize);
}

pub fn print(s: &str) {
    for c in s.bytes() {
        console_putchar(c as _);
    }
}

pub fn start_hart(hartid: usize, start_addr: usize, a1: usize) {
    //print("start_hart");
    //console_putchar(b'0' as usize + hartid);
    //print("\n");
    let ret = hart_start(hartid, start_addr, a1);
    if ret.error != RET_SUCCESS {
        panic!("start hart{} failed: {:?}", hartid, ret);
    }
    //print("end_start_hart");
    //console_putchar(b'0' as usize +hartid);
    //print("\n");
}
