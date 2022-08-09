mod cpu;
mod page_control;
mod sbi;
pub mod stdin;
pub mod stdout;

pub use sbi::{console_put_usize_in_hex, send_ipi, set_timer, shutdown, start_hart};

pub use page_control::{allow_sum_access, refuse_sum_access};

core::arch::global_asm!(include_str!("boot/entry.S"));

/// 需要在堆初始化之后，因为这里 STDOUT 打印需要用到 Mutex 锁，这需要堆分配
/// 在硬件上 start_hart 需要调用这个函数来确认启动，但是在 qemu 中，start_hart 默认是被注释掉的
#[allow(dead_code)]
pub fn cpu_init(cpu_id: usize) {
    println!("Hello, CPU [{}]", cpu_id);
}

pub fn get_cpu_id() -> usize {
    cpu::id()
}
