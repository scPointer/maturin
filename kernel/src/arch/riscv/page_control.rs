use riscv::register::sstatus;

#[allow(non_snake_case)]
pub fn allow_sum_access() {
    unsafe {
        sstatus::set_sum();
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
pub fn refuse_sum_access() {
    unsafe {
        sstatus::clear_sum();
    }
}
