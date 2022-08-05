use riscv::register::sstatus;

#[allow(non_snake_case)]
pub fn setSUMAccessOpen() {
    unsafe {
        sstatus::set_sum();
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
pub fn setSUMAccessClose() {
    unsafe {
        sstatus::clear_sum();
    }
}
