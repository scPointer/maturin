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

fn a_sbi_ecall(which:usize, fid:usize, arg0:usize, arg1: usize, arg2: usize, arg3:usize, arg4:usize, arg5:usize){
    unsafe{
        core::arch::asm!("ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            in("a6") fid,
            in("a7") which,
        );
    }
}

pub fn console_putchar(c: usize) {
    sbi_call(1, c, 0, 0);
}

pub fn console_putint(val: usize) {
    console_putchar(b'0' as usize + val);
}

pub fn print(s: &str) {
    for c in s.bytes() {
        console_putchar(c as usize);
    }
}

pub fn start_hart(hartid:usize,start_addr:usize,  a1:usize) {
    print("start_hart");
    console_putint(hartid);
    print("\n");
    a_sbi_ecall(0x48534D, 0, hartid, start_addr, a1, 0, 0, 0);
    print("end_start_hart");
    console_putint(hartid);
    print("\n");
}
