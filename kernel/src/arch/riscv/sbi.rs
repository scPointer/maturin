#![allow(dead_code)]
const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

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

fn a_sbi_ecall(
    which: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> usize {
    let ret;
    unsafe {
        core::arch::asm!("ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") which,
            lateout("a0") ret,
        );
    }
    ret
}

pub fn send_ipi(hart_mask: usize) -> usize {
    sbi_call(SBI_SEND_IPI, &hart_mask as *const _ as usize, 0, 0)
}

pub fn set_timer(stime_value: u64) {
    #[cfg(target_pointer_width = "32")]
    sbi_call(
        SBI_SET_TIMER,
        stime_value as usize,
        (stime_value >> 32) as usize,
        0,
    );
    #[cfg(target_pointer_width = "64")]
    sbi_call(SBI_SET_TIMER, stime_value as usize, 0, 0);
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!()
}

pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
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
        console_putchar(c as usize);
    }
}

pub fn start_hart(hartid: usize, start_addr: usize, a1: usize) {
    //print("start_hart");
    //console_putchar(b'0' as usize + hartid);
    //print("\n");
    let err_code = a_sbi_ecall(0x48534D, 0, hartid, start_addr, a1, 0, 0, 0);
    if err_code != 0 {
        panic!("start hart{} failed. error code={:x}", hartid, err_code);
    }
    let hart_mask = 1usize << hartid;
    let err_code = send_ipi(&hart_mask as *const _ as _);
    if err_code != 0 {
        panic!(
            "start hart{} failed to send ipi. error code={:x}",
            hartid, err_code
        );
    }
    //print("end_start_hart");
    //console_putchar(b'0' as usize +hartid);
    //print("\n");
}
