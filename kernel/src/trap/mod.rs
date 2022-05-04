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
use crate::task::{exit_current_task, suspend_current_task, handle_user_page_fault};
use crate::memory::{
    handle_kernel_page_fault,
    phys_to_virt,
    PTEFlags
};
use crate::timer::set_next_trigger;
use crate::arch::{get_cpu_id, console_put_usize_in_hex};
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    sstatus,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

pub use context::TrapContext;

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
/// 内核和用户Trap的共同入口
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {  
    match sstatus::read().spp() {
        sstatus::SPP::Supervisor => kernel_trap_handler(cx),
        sstatus::SPP::User => user_trap_handler(cx)
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn user_trap_handler(cx: &mut TrapContext) -> &mut TrapContext { 
    /*
    let mut sp: usize;
    unsafe { core::arch::asm!("mv {0}, sp", out(reg) sp) };
    println!("in sp {:x}", sp);
    println!("user sp = {:x}, entry = {:x}, sstatus = {:x}", cx.x[2], cx.sepc, cx.sstatus.bits()); 
    */

    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            //let mut pc: usize;
            //unsafe { core::arch::asm!("auipc {0}, 0", out(reg) pc) };
            //console_put_usize_in_hex(pc); 
            //println!("syscall");
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) => {
            println!("[kernel] StoreFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_task(-1);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[cpu {}] IllegalInstruction in application, kernel killed it.", get_cpu_id());
            exit_current_task(-1);
        }
        Trap::Exception(Exception::InstructionPageFault) => {
            println!("[cpu {}] InstructionPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::EXECUTE) {
                println!("{:#?}", e);
                exit_current_task(-1);
            }
            //PageFault(stval, PTEFlags::USER | PTEFlags::EXECUTE)
        }
        Trap::Exception(Exception::LoadPageFault) => {
            /*
            let mut pc: usize;
            unsafe { core::arch::asm!("auipc {0}, 0", out(reg) pc) };
            // 内部直接模拟16个位，直接用 SBI_CONSOLE_PUTCHAR 一个个打印
            console_put_usize_in_hex(pc); 
            */
            //println!("pc = {:x}", pc);

            println!("[cpu {}] LoadPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::READ) {
                println!("{:#?}", e);
                exit_current_task(-1);
            }
            //PageFault(stval, PTEFlags::USER | PTEFlags::READ)
        }
        Trap::Exception(Exception::StorePageFault) => {
            println!("[cpu {}] StorePageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::WRITE) {
                println!("{:#?}", e);
                exit_current_task(-1);
            }
            //PageFault(stval, PTEFlags::USER | PTEFlags::WRITE)
        }
        
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // println!("[cpu {}] timer interrupt", get_cpu_id());
            // 之后需要判断如果是在内核态，则不切换任务
            set_next_trigger();
            suspend_current_task();
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
    /*
    let mut sp: usize;
    unsafe { core::arch::asm!("mv {0}, sp", out(reg) sp) };
    println!("out sp {:x}", sp);
    println!("user sp = {:x}, entry = {:x}, sstatus = {:x}", cx.x[2], cx.sepc, cx.sstatus.bits()); 
    */
    cx
}

#[no_mangle]
/// handle an interrupt, exception, or system call from kernel
pub fn kernel_trap_handler(cx: &mut TrapContext) -> &mut TrapContext { 
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value

    /*
    let mut pc: usize;
    unsafe { core::arch::asm!("auipc {0}, 0", out(reg) pc) };
    let mut sp: usize;
    unsafe { core::arch::asm!("mv {0}, sp", out(reg) sp) };
    println!("pc = {:x}, sp = {:x}", pc, sp);
    */

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) => {
            error_println!("[kernel] StoreFault in kernel, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            //exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error_println!("[cpu {}] IllegalInstruction in kernel, kernel killed it.", get_cpu_id());
            //exit_current_and_run_next();
        }
        Trap::Exception(Exception::InstructionPageFault) => {
            error_println!("[cpu {}] InstructionPageFault in kernel, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            /*
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::EXECUTE) {
                println!("{:#?}", e);
                //exit_current_and_run_next();
            }
            */
            //PageFault(stval, PTEFlags::USER | PTEFlags::EXECUTE)
        }
        Trap::Exception(Exception::LoadPageFault) => {
            error_println!("[cpu {}] LoadPageFault in kernel, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            /*
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::READ) {
                println!("{:#?}", e);
                exit_current_and_run_next();
            }
            */
            //PageFault(stval, PTEFlags::USER | PTEFlags::READ)
        }
        Trap::Exception(Exception::StorePageFault) => {
            error_println!("[cpu {}] StorePageFault in kernel, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            
            /*
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::WRITE) {
                println!("{:#?}", e);
                exit_current_and_run_next();
            }
            */
            //PageFault(stval, PTEFlags::USER | PTEFlags::WRITE)
        }
        
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // 之后需要判断如果是在内核态，则不切换任务
            set_next_trigger();
            //suspend_current_and_run_next();
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
    panic!("kernel trap");
    cx
}