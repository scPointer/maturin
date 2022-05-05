#![allow(dead_code)]
pub const BOOTSTRAP_CPU_ID: usize = 0;
pub const CPU_NUM: usize =  4;
pub const LAST_CPU_ID: usize = CPU_NUM - 1;
pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
pub const CLOCK_FREQ: usize = 1250_0000; //freq for qemu -m virt
pub const IS_SINGLE_CORE: bool = false;

pub const KERNEL_STACK_SIZE: usize = 0x20_0000; // 2 MB, per CPU

//pub const MAX_APP_NUM: usize = 10; // 应用程序个数限制
//pub const APP_BASE_ADDRESS: usize = 0x8020_0000;
//pub const APP_SIZE_LIMIT: usize = 0x2_0000;
//pub const APP_ADDRESS_END: usize = APP_BASE_ADDRESS + MAX_APP_NUM * APP_SIZE_LIMIT;

pub const PAGE_SIZE: usize = 0x1000; // 4 KB
pub const PAGE_SIZE_BITS: usize = 0xc; // 4 KB = 2^12
pub const EMPTY_TASK: usize = usize::MAX;

pub const USER_STACK_SIZE: usize = 0x2000; // 8 KB,
pub const USER_STACK_OFFSET: usize = 0x4000_0000 - USER_STACK_SIZE;
pub const USER_VIRT_ADDR_LIMIT: usize = 0xFFFF_FFFF;

pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
pub const PHYS_MEMORY_OFFSET: usize = 0x8000_0000;
pub const PHYS_MEMORY_END: usize = 0x8800_0000;

pub const DEVICE_START: usize = 0x9000_0000;
pub const DEVICE_END: usize = 0x9800_0000;

/// 入口用户程序。OS启动后，只会启动以此为名的用户程序。
/// 一般来说，这个程序会通过 fork / exec 启动终端和其他程序
pub const ORIGIN_USER_PROC_NAME: &str = "start";

// 最小的 pid(进程号) 是 0，最大的 pid 是 PID_LIMIT-1
pub const PID_LIMIT: usize = 4096;
// 最大的文件描述符
pub const FD_LIMIT: usize = 256;