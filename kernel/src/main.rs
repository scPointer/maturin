#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]

mod constants;
mod lang;
mod memory;
mod console;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
mod arch;

//use core::sync::atomic::{Ordering, AtomicUsize};
use core::hint::spin_loop;

extern crate alloc;
use alloc::sync::Arc;
extern crate lock;
use lock::mutex::Mutex;

extern crate lazy_static;
//static AP_CAN_INIT: AtomicUsize = AtomicUsize::new(0);
lazy_static::lazy_static! {
    static ref AP_CAN_INIT: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
}

#[no_mangle]
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    let cpu_id = arch::cpu::id();
    if cpu_id == constants::BOOTSTRAP_CPU_ID {
        memory::init();
        //AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap();
        check_and_init(cpu_id);
    } else {
        //while cpu_id != AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap() {
        while !check_and_init(cpu_id) {
            spin_loop();
        }
    }
    
    // In fact, it is unnecessary to check all cpu booted before respective initialization
    // this is just to make a pretty output 
    while !check_all_cpu_started() {
        spin_loop();
    }
    // 
    println!("I'm CPU [{}]", cpu_id);
    /*
    for _i in 0..100 {
        arch::io::console_putint(cpu_id);
    };
    */
    
    match cpu_id {
        constants::BOOTSTRAP_CPU_ID => boot_main(),
        _ => secondary_main(),
    }
    
}

pub fn check_and_init(cpu_id: usize) -> bool {
    let mut id_now = AP_CAN_INIT.lock();
    if *id_now != cpu_id {
        false
    } else {
        arch::cpu_init(cpu_id);
        *id_now += 1;
        true
    }
}

pub fn check_all_cpu_started() -> bool {
    *AP_CAN_INIT.lock() == constants::LAST_CPU_ID + 1
}

pub fn boot_main() -> ! {
    //arch::io::print("I'm bootstrap CPU\n");
    loop {}
}

pub fn secondary_main() -> ! {
    //arch::io::print("I'm another CPU\n");
    loop {}
}
