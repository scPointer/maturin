//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].

mod context;

use crate::syscall::syscall;
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::memory::{handle_kernel_page_fault, PTEFlags};
use crate::timer::set_next_trigger;
use crate::arch::get_cpu_id;
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// timer interrupt enabled
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) => {
            println!("[kernel] StoreFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }

        // 临时的错误实现：不应该在此用handle_kernel_page_fault
        Trap::Exception(Exception::InstructionPageFault) => {
            println!("[kernel] InstructionPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", stval, cx.sepc);
            handle_kernel_page_fault(stval, PTEFlags::USER | PTEFlags::EXECUTE);
            //PageFault(stval, PTEFlags::USER | PTEFlags::EXECUTE)
        }
        Trap::Exception(Exception::LoadPageFault) => {
            println!("[kernel] LoadPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", stval, cx.sepc);
            handle_kernel_page_fault(stval, PTEFlags::USER | PTEFlags::READ);
            //PageFault(stval, PTEFlags::USER | PTEFlags::READ)
        }
        Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] StorePageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", stval, cx.sepc);
            //panic!("..");
            if cx.sepc == 0xffff_ffff_8020_999a {
                panic!("...");
            }
            handle_kernel_page_fault(stval, PTEFlags::USER | PTEFlags::WRITE);
            //PageFault(stval, PTEFlags::USER | PTEFlags::WRITE)
        }
        
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "[cpu {}] Unsupported trap {:?}, stval = {:#x}!",
                get_cpu_id(),
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::TrapContext;
