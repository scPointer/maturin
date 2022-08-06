mod cpu;
mod page_control;
mod sbi;
pub mod stdin;
pub mod stdout;

use core::mem::MaybeUninit;

pub use page_control::{allow_sum_access, refuse_sum_access};
pub use sbi::{console_put_usize_in_hex, send_ipi, set_timer, shutdown, start_hart};

core::arch::global_asm!(
    "   .section .data
        .align 12
    boot_page_table_sv39:
        .quad 0
        .quad 0
        # 0x00000000_80000000 -> 0x80000000 (1G, VRWXAD)
        .quad (0x80000 << 10) | 0xcf
        # removed
        #.quad 0
        .zero 8 * 507
        # 0xffffffff_80000000 -> 0xffffffff_80000000 (1G, VRWXAD)
        .quad (0x80000 << 10) | 0xcf
        .quad 0
    "
);

/// 一个核的启动栈
#[repr(C, align(4096))]
struct KernelStack([u8; 256 * 1024]);

/// 所有核的启动栈
#[link_section = ".bss.stack"]
static mut KERNEL_STACK: MaybeUninit<[KernelStack; 4]> = MaybeUninit::uninit();

/// 获取启动栈地址
#[inline]
pub fn kernel_stack() -> core::ops::Range<usize> {
    let base = unsafe { KERNEL_STACK.assume_init_ref() } as *const _ as usize;
    let size = core::mem::size_of_val(unsafe { &KERNEL_STACK });
    base..base + size
}

/// 入口。
///
/// # Safety
///
/// 裸函数。
#[naked]
#[link_section = ".text.entry"]
#[export_name = "_start"]
unsafe extern "C" fn entry(hartid: usize) -> ! {
    core::arch::asm!(
        "   mv   tp, a0",
        "   call {set_stack}",
        "   call {set_boot_pt}",
        // jump to start_kernel
        "   la   t0, start_kernel
            li   t1, 0xffffffff00000000
            add  t0, t0, t1
            add  sp, sp, t1
            jr   t0
        ",
        set_stack   = sym set_stack,
        set_boot_pt = sym set_boot_pt,
        options(noreturn),
    )
}

/// 副核入口。
///
/// # Safety
///
/// 裸函数。
#[naked]
pub unsafe extern "C" fn secondary_entry(hartid: usize) -> ! {
    core::arch::asm!(
        "   mv   tp, a0",
        "   call {set_stack}",
        "   call {set_boot_pt}",
        // jump to start_kernel
        "   la   t0, start_kernel_secondary
            li   t1, 0xffffffff00000000
            add  t0, t0, t1
            add  sp, sp, t1
            jr   t0
        ",
        set_stack = sym set_stack,
        set_boot_pt = sym set_boot_pt,
        options(noreturn),
    )
}

/// 设置启动栈
#[naked]
unsafe extern "C" fn set_stack(hartid: usize) {
    core::arch::asm!(
        "   add  t0, a0, 1
            slli t0, t0, 18
            la   sp, {stack}
            add  sp, sp, t0
            ret
        ",
        stack = sym KERNEL_STACK,
        options(noreturn),
    )
}

/// 设置启动页表
#[naked]
unsafe extern "C" fn set_boot_pt(hartid: usize) {
    core::arch::asm!(
        "   la   t0, boot_page_table_sv39
            srli t0, t0, 12
            li   t1, 8 << 60
            or   t0, t0, t1
            csrw satp, t0
            ret
        ",
        options(noreturn),
    )
}

/// 需要在堆初始化之后，因为这里 STDOUT 打印需要用到 Mutex 锁，这需要堆分配
/// 在硬件上 start_hart 需要调用这个函数来确认启动，但是在 qemu 中，start_hart 默认是被注释掉的
#[allow(dead_code)]
pub fn cpu_init(cpu_id: usize) {
    println!("Hello, CPU [{}]", cpu_id);
}

pub fn get_cpu_id() -> usize {
    cpu::id()
}
