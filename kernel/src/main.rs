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

use core::sync::atomic::{Ordering, AtomicBool, AtomicUsize};
use core::hint::spin_loop;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate alloc;
use alloc::sync::Arc;

extern crate lock;
use lock::mutex::Mutex;

extern crate lazy_static;

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref BOOTED_CPU_NUM: AtomicUsize = AtomicUsize::new(0);
}

#[no_mangle]
/// 启动OS，每个核分别执行用户程序
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    let cpu_id = arch::get_cpu_id();
    if cpu_id == constants::BOOTSTRAP_CPU_ID {
        memory::clear_bss();
        memory::allocator_init();
        memory::kernel_page_table_init();
        println!("[CPU {}] page table enabled", cpu_id);
        trap::init();

        //risc-v ISA manual 建议尽量不设SUM位，防止内核不小心修改用户地址中的内容
        //所以现在默认不开启这个权限
        //只在需要读取用户地址空间的数据(如系统调用 sys_write)时开启

        //arch::setSUMAccessOpen(); //开启内核直接访问用户地址空间的权限
        //loader::load_apps();
        //arch::setSUMAccessClose();

        trap::enable_timer_interrupt();
        timer::set_next_trigger();
        
        //AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap();
        first_cpu_bootstrap_finish(cpu_id);

    } else {
        //while cpu_id != AP_CAN_INIT.compare_exchange(cpu_id, cpu_id + 1, Ordering::Relaxed, Ordering::Relaxed).unwrap() {
        while !first_cpu_bootstrap_finish(cpu_id) {
            spin_loop();
        }
        memory::kernel_page_table_init();
        trap::init();
        trap::enable_timer_interrupt();
        timer::set_next_trigger();
    }
    
    // 等待所有核启动完成
    // 这一步是为了进行那些**需要所有CPU都启动后才能进行的全局初始化操作**
    // 然而目前还没有这样的操作，所以现在这里只是用来展示无锁的原子变量操作(参见下面两个函数)
    bootstrap_finish();
    while !check_all_cpu_started() {
        spin_loop();
    }
    println!("I'm CPU [{}]", cpu_id);

    // 全局初始化结束
    
    match cpu_id {
        constants::BOOTSTRAP_CPU_ID => boot_main(),
        _ => secondary_main(),
    }
    
}

/// 检查第一个核是否初始化完成，如是则返回 true
/// 内核必须由 cpu_id 等于 AP_CAN_INIT 初始值的核先启动，然后其他的核才能启动 
pub fn first_cpu_bootstrap_finish(cpu_id: usize) -> bool {
    if cpu_id == constants::BOOTSTRAP_CPU_ID {
        AP_CAN_INIT.compare_exchange(false, true, Ordering::Release, Ordering::Relaxed).unwrap();
        true
    } else {
        AP_CAN_INIT.load(Ordering::Acquire)
    }
}

/// 确认当前核已启动(BOOTSTRAP_CPU 也需要调用)
pub fn bootstrap_finish() {
    BOOTED_CPU_NUM.fetch_add(1, Ordering::Relaxed);
}

/// 检查是否所有核已启动，如是则返回 true
pub fn check_all_cpu_started() -> bool {
    BOOTED_CPU_NUM.load(Ordering::Relaxed) == constants::CPU_NUM
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
