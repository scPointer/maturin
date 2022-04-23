//! Loading user applications into memory
//!
//! For chapter 3, user applications are simply part of the data included in the
//! kernel binary, so we only need to copy them to the space allocated for each
//! app to load them. We also allocate fixed spaces for each task's
//! [`KernelStack`] and [`UserStack`].

use lazy_static::*;
use alloc::vec::Vec;

use crate::constants::{
    PAGE_SIZE,
    USER_STACK_SIZE,
    KERNEL_STACK_SIZE,
    MAX_APP_NUM,
    APP_BASE_ADDRESS,
    APP_SIZE_LIMIT,
    PHYS_VIRT_OFFSET,
};
use crate::trap::TrapContext;
use crate::memory::Frame;
use core::arch::asm;

//#[repr(align(4096))]
pub struct KernelStack {
    frame: Frame,
    //data: [u8; KERNEL_STACK_SIZE],
}

//#[repr(align(4096))]
pub struct UserStack {
    frame: Frame,
    //data: [u8; USER_STACK_SIZE],
}

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.frame.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, trap_cx: TrapContext) -> usize {
        let trap_cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        trap_cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.frame.as_ptr() as usize + USER_STACK_SIZE
    }
}

lazy_static! {
    pub static ref KERNEL_STACK: Vec<KernelStack> = {
        let mut stacks: Vec<KernelStack> = Vec::new();
        for i in 0..MAX_APP_NUM {
            stacks.push(KernelStack {
                frame: Frame::new_contiguous(KERNEL_STACK_SIZE / PAGE_SIZE, 9).unwrap(),
                // data: [0; KERNEL_STACK_SIZE],
            })
        }
        stacks
    };

    pub static ref USER_STACK: Vec<UserStack> = {
        let mut stacks: Vec<UserStack> = Vec::new();
        for _i in 0..MAX_APP_NUM {
            stacks.push(UserStack {
                frame: Frame::new_contiguous(USER_STACK_SIZE / PAGE_SIZE, 9).unwrap(),
                // data: [0; KERNEL_STACK_SIZE],
            })
        }
        stacks
    };
}

/// Get base address of app i.
fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// Load nth user app at
/// [APP_BASE_ADDRESS + n * APP_SIZE_LIMIT, APP_BASE_ADDRESS + (n+1) * APP_SIZE_LIMIT).
pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    println!("num_app_ptr {:x}, num_app {}", num_app_ptr as usize, num_app);
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    // clear i-cache first
    unsafe {
        asm!("fence.i");
    }
    // load apps
    for i in 0..num_app {
        let base_i = get_base_i(i);
        println!("load {}, base {:x} app_origin_pos {:x}", i, base_i, app_start[i] as usize);
        //let ad :usize = 0x8010_0000;
        //unsafe { (ad as *mut u8).write_volatile(0)}
        //println!("written");

        // clear region
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
        
        println!("load {}", i);
        // load app from data section to memory
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        dst.copy_from_slice(src);
        println!("load {}", i);
    }
}

/// get applications data
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

/// get app info with entry and sp and save `TrapContext` in kernel stack
pub fn init_app_cx(app_id: usize) -> usize {
    println!("user stack bottom at {:x}", USER_STACK[app_id].get_sp());
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        get_base_i(app_id),
        USER_STACK[app_id].get_sp(),
    ))
}

/// 同 init_app_cx，但由外部给出用户栈和程序入口
/// 一般是由其他程序调用 get_app_data() 分析后再调用这个函数
pub fn init_app_cx_by_entry_and_stack(app_id: usize, user_entry: usize, user_stack: usize) -> usize{
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        user_entry,
        user_stack,
    ))
}