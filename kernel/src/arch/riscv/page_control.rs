use riscv::register::sstatus;

pub fn setSUMAccessOpen() {
    unsafe {sstatus::set_sum(); }
}

pub fn setSUMAccessClose() {
    unsafe { sstatus::clear_sum(); }
}