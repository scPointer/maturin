//! 中断异常处理
//!
//! 所有中断和异常的入口在 trap.S 中的 __alltraps，它会在保存上下文信息后跳转到本文件中的 trap_handler 函数
//!
//! 在这个模块中，程序的执行流不一定正常。主要有三种可能：
//!
//! 1. 用户程序中断：进入 __alltraps
//!  -> 调用 trap_handler
//!  -> trap_handler 返回到 __restore
//!
//! 2. 第一次进入用户程序：生成一个 KernelStack，在栈顶构造一个 TrapContext
//!  -> 设置 sp 为这个栈的栈顶
//!  -> 直接跳转到 __restore，假装它是 trap_handler 返回的
//!
//! 3. sys_exec 生成的用户程序：进入 __alltraps
//!  -> 调用 trap_handler
//!  -> 重写 KernelStack 栈顶的 TrapContext（不通过 trap_handler 的参数，而是直接写对应内存）
//!  -> 和上一种情况一样，直接跳到 __restore

//#![deny(missing_docs)]

mod context;

use crate::{
    arch::get_cpu_id,
    constants::SIGNAL_RETURN_TRAP,
    memory::PTEFlags,
    signal::{SignalNo, send_signal},
    syscall::syscall,
    task::{
        handle_signals,
        handle_user_page_fault,
        suspend_current_task,
        get_current_task,
        timer_kernel_to_user,
        timer_user_to_kernel,
        signal_return,
    },
    timer::set_next_trigger,
};
use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, sstatus, stval, stvec,
};

pub use context::TrapContext;

global_asm!(include_str!("trap.S"));

/// 设置寄存器 stvec 指向 __alltraps，它定义在 trap.S 中
pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// 打开时间中断
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
/// 内核和用户Trap的共同入口
///
/// 参数 cx 是触发中断的程序的上下文信息，它在 trap.S 里被压在内核栈中。
/// 注意，因为我们的实现没有一个专门的 "trap栈"，所以你可以认为进入该函数时 cx 就在 sp 的"脚底下"。
/// 所以修改 cx 时一旦越界就可能改掉该函数的 ra/sp，要小心。
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    match sstatus::read().spp() {
        sstatus::SPP::Supervisor => kernel_trap_handler(cx),
        sstatus::SPP::User => {
            timer_user_to_kernel();
            let cx = user_trap_handler(cx);
            timer_kernel_to_user();
            cx
        }
    }
}

#[no_mangle]
/// 处理来自用户程序的异常/中断
pub fn user_trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    
    //if get_current_task().unwrap().get_tid_num() == 2 {
    //let mut fs1: f64;
    //unsafe { core::arch::asm!("fmv.d.x {0}, fs1", out(reg) fs1) };
    //println!("in fs1 {}", fs1);
    //unsafe { core::arch::asm!("fsd fs1, 0(sp)") };
    //println!("user sp = {:x}, entry = {:x}, sstatus = {:x}", cx.x[2], cx.sepc, cx.sstatus.bits());
    //}
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    timer_user_to_kernel();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            //let mut pc: usize;
            //unsafe { core::arch::asm!("auipc {0}, 0", out(reg) pc) };
            //console_put_usize_in_hex(pc);
            //println!("syscall");
            cx.sepc += 4;
            cx.x[10] = syscall(
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]],
            ) as usize;
        }
        Trap::Exception(Exception::StoreFault) => {
            info!("[kernel] StoreFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            send_signal(get_current_task().unwrap().get_tid_num(), SignalNo::SIGSEGV as usize);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            info!("[cpu {}] IllegalInstruction in application, sepc = {:x}, stval = {:#x}, kernel killed it.", get_cpu_id(), cx.sepc, stval);
            send_signal(get_current_task().unwrap().get_tid_num(), SignalNo::SIGSEGV as usize);
        }
        Trap::Exception(Exception::InstructionPageFault) => {
            info!("[cpu {}] InstructionPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            if stval == SIGNAL_RETURN_TRAP {
                // 当作调用了 sigreturn 一样
                cx.x[10] = signal_return() as usize;
                return cx;
            }
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::EXECUTE) {
                info!("{:#?}", e);
                send_signal(get_current_task().unwrap().get_tid_num(), SignalNo::SIGSEGV as usize);
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

            //info!("[cpu {}] LoadPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::READ) {
                info!("[cpu {}] LoadPageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
                info!("{:#?}", e);
                send_signal(get_current_task().unwrap().get_tid_num(), SignalNo::SIGSEGV as usize);
            }
            //PageFault(stval, PTEFlags::USER | PTEFlags::READ)
        }
        Trap::Exception(Exception::StorePageFault) => {
            //info!("[cpu {}] StorePageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::WRITE) {
                info!("[cpu {}] StorePageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
                info!("{:#?}", e);
                send_signal(get_current_task().unwrap().get_tid_num(), SignalNo::SIGSEGV as usize);
            }
            //PageFault(stval, PTEFlags::USER | PTEFlags::WRITE)
        }

        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // println!("[cpu {}] timer interrupt", get_cpu_id());
            println!(
                "[cpu {}] timer interrupt, sepc = {:#x}",
                get_cpu_id(),
                cx.sepc
            );

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
    handle_signals();
    /*
    let mut sp: usize;
    unsafe { core::arch::asm!("mv {0}, sp", out(reg) sp) };
    println!("out sp {:x}", sp);
    println!("user sp = {:x}, entry = {:x}, sstatus = {:x}", cx.x[2], cx.sepc, cx.sstatus.bits());
    */
    cx
}

#[no_mangle]
/// 处理来自内核的异常/中断
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
            cx.x[10] = syscall(
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]],
            ) as usize;
        }
        Trap::Exception(Exception::StoreFault) => {
            eprintln!("[kernel] StoreFault in kernel, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            //exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            eprintln!(
                "[cpu {}] IllegalInstruction in kernel, kernel killed it.",
                get_cpu_id()
            );
            //exit_current_and_run_next();
        }
        Trap::Exception(Exception::InstructionPageFault) => {
            eprintln!("[cpu {}] InstructionPageFault in kernel, bad addr = {:#x}, bad instruction = {:#x}.", get_cpu_id(), stval, cx.sepc);
            /*
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::EXECUTE) {
                println!("{:#?}", e);
                //exit_current_and_run_next();
            }
            */
            //PageFault(stval, PTEFlags::USER | PTEFlags::EXECUTE)
        }
        Trap::Exception(Exception::LoadPageFault) => {
            eprintln!(
                "[cpu {}] LoadPageFault in kernel, bad addr = {:#x}, bad instruction = {:#x}.",
                get_cpu_id(),
                stval,
                cx.sepc
            );
            /*
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::READ) {
                println!("{:#?}", e);
                exit_current_and_run_next();
            }
            */
            //PageFault(stval, PTEFlags::USER | PTEFlags::READ)
        }
        Trap::Exception(Exception::StorePageFault) => {
            eprintln!(
                "[cpu {}] StorePageFault in kernel, bad addr = {:#x}, bad instruction = {:#x}.",
                get_cpu_id(),
                stval,
                cx.sepc
            );

            /*
            if let Err(e) = handle_user_page_fault(stval, PTEFlags::USER | PTEFlags::WRITE) {
                println!("{:#?}", e);
                exit_current_and_run_next();
            }
            */
            //PageFault(stval, PTEFlags::USER | PTEFlags::WRITE)
        }

        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            println!(
                "[cpu {}] timer interrupt(KERNEL), sepc = {:#x}",
                get_cpu_id(),
                cx.sepc
            );
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
}
