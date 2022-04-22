//! 程序入口为 mod arch 中的 entry.S
//! 将cpu_id存到tp寄存器并设置好初始的内核栈与页表后，跳转到 start_kernel 启动
#![no_std]
#![no_main]
#![deny(missing_docs)]
//#![deny(warnings)]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]

#[macro_use]
mod console;

mod constants;
mod lang;
mod memory;
mod loader;
mod timer;
mod error;
mod loaders;

pub mod syscall;
pub mod task;
pub mod trap;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
mod arch;

//在引入 mod arch 时已经加入了 entry.S
core::arch::global_asm!(include_str!("link_app.S"));

//use core::sync::atomic::{Ordering, AtomicUsize};
use core::hint::spin_loop;

#[macro_use]
extern crate bitflags;

#[macro_use]
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
/// 启动OS，每个核分别执行用户程序
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    let cpu_id = arch::get_cpu_id();
    if cpu_id == constants::BOOTSTRAP_CPU_ID {
        //memory::clear_bss();
        memory::allocator_init();
        memory::kernel_page_table_init();
        println!("[CPU {}] page table enabled", cpu_id);
        trap::init();

        arch::setSUMAccessOpen(); //开启内核直接访问用户地址空间的权限
        loader::load_apps();
        //arch::setSUMAccessClose();

        trap::enable_timer_interrupt();
        timer::set_next_trigger();
        
        //AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap();
        check_and_finish_init(cpu_id);

    } else {
        //while cpu_id != AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap() {
        while !check_and_finish_init(cpu_id) {
            spin_loop();
        }
        memory::kernel_page_table_init();
        trap::init();
        trap::enable_timer_interrupt();
        timer::set_next_trigger();
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

/// 检查是否*可以*初始化，如可则返回 true
/// 这个函数制定了内核必须由 cpu_id 等于 AP_CAN_INIT 初始值的核先启动，然后启动的 cpu_id 依次递增
/// 也即如有 N 个核启动，要求其 cpu_id 必须为 [0,N-1]
pub fn check_and_finish_init(cpu_id: usize) -> bool {
    let mut id_now = AP_CAN_INIT.lock();
    if *id_now != cpu_id {
        false
    } else {
        arch::cpu_init(cpu_id);
        *id_now += 1;
        true
    }
}

/// 检查是否所有核已启动，如是则返回 true
pub fn check_all_cpu_started() -> bool {
    *AP_CAN_INIT.lock() == constants::LAST_CPU_ID + 1
}

/// 第一个核启动后的任务
pub fn boot_main() -> ! {
    //arch::io::print("I'm bootstrap CPU\n");
    test_vm();
    task::run_first_task();
    loop {}
}

/// 其他核启动后的任务
pub fn secondary_main() -> ! {
    //arch::io::print("I'm another CPU\n");
    task::run_first_task();
    loop {}
}

/// 输出linker各段的虚存映射
pub fn test_vm() {
    extern "C" {
        fn stext();
        fn etext();
        fn sdata();
        fn edata();
        fn srodata();
        fn erodata();
        fn sbss();
        fn ebss();
    }
    println!("stext = {:x}\netext = {:x}\nsdata = {:x}\nedata = {:x}\nsrodata = {:x}\nerodata = {:x}\nsbss = {:x}\nebss = {:x}\n",
        stext as usize, etext as usize, sdata as usize, edata as usize, srodata as usize, erodata as usize, sbss as usize, ebss as usize);
}
