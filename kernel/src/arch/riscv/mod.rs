mod cpu;
mod sbi;
pub mod stdin;
pub mod stdout;
mod page_control;

pub use sbi::{
    set_timer,
    shutdown,
    console_put_usize_in_hex,
};

pub use page_control::{
    setSUMAccessClose,
    setSUMAccessOpen,
};

core::arch::global_asm!(include_str!("boot/entry.S"));

/// 需要在堆初始化之后，因为这里 STDOUT 打印需要用到 Mutex 锁，这需要堆分配
pub fn cpu_init(cpu_id: usize) {
    println!("Hello, CPU [{}]", cpu_id);
    /*
    sbi::print("Hello, CPU [");
    sbi::console_putint(cpu_id);
    sbi::print("]\n");
    */
}

pub fn get_cpu_id() -> usize {
    cpu::id()
}
