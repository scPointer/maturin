//! SUM Access for Supervisor User Memory Access

use riscv::register::sstatus;

#[inline]
pub fn allow_sum_access() {
    unsafe { sstatus::set_sum() };
}

#[allow(unused)]
#[inline]
pub fn refuse_sum_access() {
    unsafe { sstatus::clear_sum() };
}
