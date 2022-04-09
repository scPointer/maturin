#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]

mod constants;
mod lang;
mod memory;

//#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
mod arch;

//use core::sync::atomic::{Ordering, AtomicUsize};
use core::sync::atomic::{AtomicBool, Ordering};
use core::hint::spin_loop;

use crate::memory::clear_bss;
extern crate alloc;
extern crate lock;

extern crate lazy_static;
// lazy_static::lazy_static! {
//     static ref AP_CAN_INIT: Arc<Mutex<usize>> = Arc::new(Mutex::new(1));
// }

#[no_mangle]
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);
    arch::io::print("Hello");    
    let cpu_id = arch::cpu::id();
    arch::io::console_putint(cpu_id);
    if cpu_id == constants::BOOTSTRAP_CPU_ID {
        clear_bss();
        memory::init();
        // AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap();
        AP_CAN_INIT.store(true, Ordering::Release);
        arch::io::console_putint(cpu_id);
    } else {
        //while cpu_id != AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap() {
        while !AP_CAN_INIT.load(Ordering::Acquire){
            spin_loop();
        }
        arch::io::console_putint(cpu_id);
    }
    loop {}
    // // In fact, it is unnecessary to check all cpu booted before respective initialization
    // // this is just to make a pretty output 
    // while !check_all_cpu_started() {
    //     spin_loop();
    // }
    // // 

    // for _i in 0..100 {
    //     arch::io::console_putint(cpu_id);
    // };

    // match cpu_id {
    //     constants::BOOTSTRAP_CPU_ID => boot_main(),
    //     _ => secondary_main(),
    // }
    
}

pub fn boot_main() -> ! {
    //arch::io::print("I'm bootstrap CPU\n");
    loop {}
}

pub fn secondary_main() -> ! {
    //arch::io::print("I'm another CPU\n");
    loop {}
}
