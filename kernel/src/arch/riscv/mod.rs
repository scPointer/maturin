mod cpu;
mod sbi;
pub mod stdout;

pub use sbi::{
    set_timer,
    shutdown,   
};

core::arch::global_asm!(include_str!("boot/entry.S"));

pub fn cpu_init(cpu_id: usize) {
    sbi::print("Hello, CPU [");
    sbi::console_putint(cpu_id);
    sbi::print("]\n");
}

pub fn get_cpu_id() -> usize {
    cpu::id()
}
