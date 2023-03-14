//! 程序入口为 mod arch/riscv/mod.rs 中的函数 entry ( 也是build.rs中的ENTRY(_start) ),
//! 函数 entry 将cpu_id存到tp寄存器并设置好初始的内核栈与页表后，跳转到 start_kernel 启动

#![no_std]
#![no_main]
#![deny(warnings)]
#![feature(panic_info_message)]
#![feature(naked_functions, asm_const)]
// MemorySet 处理相交的 VmArea 时需要
#![feature(btree_drain_filter)]
// test.rs 输入 argv 需要
#![feature(drain_filter)]

#[macro_use]
extern crate log;

#[macro_use]
pub mod console;
pub mod constants;
pub mod drivers;
pub mod error;
pub mod file;
pub mod lang;
pub mod loaders;
pub mod memory;
pub mod signal;
pub mod syscall;
pub mod task;
pub mod testcases;
pub mod trap;
pub mod utils;

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

use alloc::sync::Arc;
use base_file::File;
// use core::sync::atomic::AtomicUsize;
// static BOOTED_CPU_NUM: AtomicUsize = AtomicUsize::new(0);
use log::*;

struct TaskTrampoline;

impl task_trampoline::TaskTrampoline for TaskTrampoline {
    fn suspend_current_task(&self) {
        task::suspend_current_task()
    }

    fn get_file(&self, fd: usize) -> Option<Arc<dyn base_file::File>> {
        let task = task::get_current_task().unwrap();
        let fd_manager = task.fd_manager.lock();
        if let Ok(file) = fd_manager.get_file(fd) {
            Some(file)
        } else {
            None
        }
    }

    fn push_file(&self, file: Arc<dyn File>) -> Result<usize, u64> {
        let task = task::get_current_task().unwrap();
        let mut fd_manager = task.fd_manager.lock();
        // TODO: 考虑是否要将错误类型返回给用户
        fd_manager.push(file).map_err(|_| 1)
    }

    fn manually_alloc_user_str(&self, buf: *const u8, len: usize) -> Result<(), u64> {
        let task = task::get_current_task().unwrap();
        let mut task_vm = task.vm.lock();
        // TODO: 考虑是否要将错误类型返回给用户
        task_vm.manually_alloc_user_str(buf, len).map_err(|_| 1)
    }

    fn manually_alloc_range(&self, start_vaddr: usize, end_vaddr: usize) -> Result<(), u64> {
        let task = task::get_current_task().unwrap();
        let mut task_vm = task.vm.lock();
        // TODO: 考虑是否要将错误类型返回给用户
        task_vm
            .manually_alloc_range(start_vaddr, end_vaddr)
            .map_err(|_| 1)
    }

    fn raw_time(&self) -> (usize, usize) {
        let task = task::get_current_task().unwrap();
        let time = task.time.lock();
        time.output_raw()
    }

    fn raw_timer(&self) -> (usize, usize) {
        let task = task::get_current_task().unwrap();
        let time = task.time.lock();
        time.output_raw_timer()
    }

    fn set_timer(
        &self,
        timer_interval_us: usize,
        timer_remained_us: usize,
        timer_type: usize,
    ) -> bool {
        let task = task::get_current_task().unwrap();
        let mut time = task.time.lock();
        time.set_raw_timer(timer_interval_us, timer_remained_us, timer_type)
    }
}

#[no_mangle]
/// 主核启动OS
pub extern "C" fn start_kernel(_arg0: usize, _arg1: usize) -> ! {
    arch::clear_bss(); // 清空 bss 段
    console::init_logger(crate::constants::LOG_LEVEL).unwrap();
    task_trampoline::init_task_trampoline(&TaskTrampoline);
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

#[no_mangle]
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
