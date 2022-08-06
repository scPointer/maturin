pub fn id() -> usize {
    let cpu_id;
    unsafe { core::arch::asm!("mv {0}, tp", out(reg) cpu_id) };
    cpu_id
}
