//! 程序入口为 mod arch 中的 entry.S
//! 将cpu_id存到tp寄存器并设置好初始的内核栈与页表后，跳转到 start_kernel 启动

#![no_std]
#![no_main]
#![deny(warnings)]
#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(naked_functions, asm_sym, asm_const)]
#![feature(const_btree_new)]
// MemorySet 处理相交的 VmArea 时需要
#![feature(btree_drain_filter)]
// test.rs 输入 argv 需要
#![feature(drain_filter)]

#[macro_use]
mod console;
mod constants;
mod drivers;
mod error;
mod ffi;
mod file;
mod lang;
mod loaders;
mod memory;
mod signal;
pub mod syscall;
pub mod task;
mod timer;
pub mod trap;

// #[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
mod arch;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate alloc;
extern crate fatfs;
extern crate fscommon;
extern crate lock;

mod fsio {
    pub use fscommon::{Read, Seek, Write};
}

core::arch::global_asm!(include_str!("fs.S"));

// use core::sync::atomic::AtomicUsize;
// static BOOTED_CPU_NUM: AtomicUsize = AtomicUsize::new(0);

/// 主核启动OS
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    memory::clear_bss(); // 清空 bss 段
    memory::allocator_init(); // 初始化堆分配器和页帧分配器

    memory::enable_kernel_page_table(); // 构造并切换到内核态页表与 MemorySet
    trap::init(); // 设置异常/中断的入口，即 stvec
    arch::allow_sum_access(); // 内核可以读写 USER 页表项中的数据

    //trap::enable_timer_interrupt(); // 开启时钟中断
    //timer::set_next_trigger(); // 设置时钟中断频率

    // file::list_apps_names_at_root_dir(); // 展示所有用户程序的名字
    file::list_files_at_root(); // 展示所有用户程序的名字
    file::fs_init(); // 初始化一些不是实际文件本身但是 OS 约定需要的"文件"
    let cpu_id = arch::get_cpu_id();
    info!("CPU [{cpu_id}] bootstrap");
    for other_cpu in constants::FIRST_CPU_ID..=constants::LAST_CPU_ID {
        if other_cpu != cpu_id {
            let _entry = arch::secondary_entry as usize;
            // println!("other_cpu {}", other_cpu);
            // arch::start_hart(other_cpu, memory::virt_to_phys(_entry), 0);
        }
    }

    // let cpu_id = arch::get_cpu_id();
    //println!("CPU [{}] is waiting", cpu_id);

    // 全局初始化结束
    if constants::SPIN_LOOP_AFTER_BOOT {
        loop {}
    } else {
        task::run_tasks();
    }
}

/// 其他核启动OS
pub extern "C" fn start_kernel_secondary(_arg0: usize, _arg1: usize) -> ! {
    memory::enable_kernel_page_table(); // 构造并切换到内核态页表与 MemorySet
    trap::init(); // 设置异常/中断的入口，即 stvec
    arch::allow_sum_access(); // 修改 sstatus 的 SUM 位，使内核可以读写USER页表项中的数据
                              //trap::enable_timer_interrupt(); // 开启时钟中断
                              //timer::set_next_trigger(); // 设置时钟中断频率

    let cpu_id = arch::get_cpu_id();
    info!("I'm CPU [{cpu_id}]");

    // 全局初始化结束
    if constants::SPIN_LOOP_AFTER_BOOT || constants::IS_SINGLE_CORE {
        loop {}
    } else {
        task::run_tasks();
    }
}

/*
/// 是否已经有核在进行全局初始化
//static GLOBAL_INIT_STARTED: AtomicBool = AtomicBool::new(false);
/// 全局初始化是否已结束
//static GLOBAL_INIT_FINISHED: AtomicBool = AtomicBool::new(false);

/// 是否还没有核进行全局初始化，如是则返回 true
fn can_do_global_init() -> bool {
    // GLOBAL_INIT_STARTED.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    /* match arch::get_cpu_id() {
        BOOTSTRAP_CPU_ID => {
            GLOBAL_INIT_STARTED.store(true, Ordering::Release);
            true
        },
        _ => false
    }
    */
    if GLOBAL_INIT_STARTED.load(Ordering::Acquire) == false {
        GLOBAL_INIT_STARTED.store(true, Ordering::Release);
        true
    } else {
        false
    }
}

/// 标记那些全局只执行一次的启动步骤已完成。
/// 内核必须由 cpu_id 等于 AP_CAN_INIT 初始值的核先启动并执行这些全局只需要一次的操作，然后其他的核才能启动
fn mark_global_init_finished() {
    // GLOBAL_INIT_FINISHED.compare_exchange(false, true, Ordering::Release, Ordering::Relaxed).unwrap();
    GLOBAL_INIT_FINISHED.store(true, Ordering::Release);
}

/// 等待那些全局只执行一次的启动步骤是否完成
fn wait_global_init_finished() {
    while GLOBAL_INIT_FINISHED.load(Ordering::Acquire) == false {
        spin_loop();
    }
}

/// 确认当前核已启动(BOOTSTRAP_CPU 也需要调用)
fn mark_bootstrap_finish() {
    BOOTED_CPU_NUM.fetch_add(1, Ordering::Release);
}

/// 等待所有核已启动
fn wait_all_cpu_started() {
    while BOOTED_CPU_NUM.load(Ordering::Acquire) < constants::LAST_CPU_ID - constants::FIRST_CPU_ID + 1 && !constants::IS_SINGLE_CORE {
        spin_loop();
    }
}
*/

/// 测试输出linker各段的虚存映射
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
    println!(
        "\
stext = {:x}
etext = {:x}
sdata = {:x}
edata = {:x}
srodata = {:x}
erodata = {:x}
sbss = {:x}
ebss = {:x}
",
        stext as usize,
        etext as usize,
        sdata as usize,
        edata as usize,
        srodata as usize,
        erodata as usize,
        sbss as usize,
        ebss as usize,
    );
}
