#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        eprintln!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        eprintln!("[kernel] Panicked: {}", info.message().unwrap());
    }
    crate::arch::shutdown_failure()
}
