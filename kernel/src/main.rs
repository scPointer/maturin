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

use crate::arch::io::start_hart;
use crate::constants::BOOTSTRAP_CPU_ID;

extern crate alloc;
extern crate lock;

extern crate lazy_static;
// lazy_static::lazy_static! {
//     static ref AP_CAN_INIT: Arc<Mutex<usize>> = Arc::new(Mutex::new(1));
// }

#[no_mangle]
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    extern {
        fn _start();
    }
    static AP_CAN_INIT: AtomicBool = AtomicBool::new(true);
    static mut booted:[bool;5] = [false, false, false, false, false];
    // arch::io::print("Hello");    
    let cpu_id = arch::cpu::id();
    // arch::io::console_putint(cpu_id);
    if AP_CAN_INIT.load(Ordering::Acquire) {
        AP_CAN_INIT.store(false, Ordering::Release);
        memory::clear_bss();
        memory::init();
        arch::io::console_putint(cpu_id);
        arch::io::print("\n");
        unsafe{
            booted[cpu_id] = true;
        }
        
        for i in 1..5{
            if i == cpu_id{
                continue;
            }
            arch::io::print("before start_hart");
            arch::io::console_putint(cpu_id);
            arch::io::print("\n");
            start_hart(i, _start as usize, 0);
            unsafe{
                while !booted[i]{

                }                
            }

        }
        
    } else {
        arch::io::console_putint(cpu_id);
        unsafe{
            booted[cpu_id] = true;
        }
        
    }
    
    // // In fact, it is unnecessary to check all cpu booted before respective initialization
    // // this is just to make a pretty output 
    // while !check_all_cpu_started() {
    //     spin_loop();
    // }
    // // 

    for _i in 0..100 {
        arch::io::console_putint(cpu_id);
    };

    // match cpu_id {
    //     constants::BOOTSTRAP_CPU_ID => boot_main(),
    //     _ => secondary_main(),
    // }
    loop{}
}

pub fn boot_main() -> ! {
    //arch::io::print("I'm bootstrap CPU\n");
    loop {}
}

pub fn secondary_main() -> ! {
    //arch::io::print("I'm another CPU\n");
    loop {}
}
