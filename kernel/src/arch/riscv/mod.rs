pub mod cpu;
pub mod io;

core::arch::global_asm!(include_str!("boot/entry.S"));

pub fn cpu_init(cpu_id: usize) {
    io::print("Hello, CPU [");
    io::console_putint(cpu_id);
    io::print("]\n");
}

