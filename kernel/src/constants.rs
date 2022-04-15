#![allow(dead_code)]
pub const BOOTSTRAP_CPU_ID: usize = 0;
pub const CPU_NUM: usize =  4;
pub const LAST_CPU_ID: usize = CPU_NUM - 1;
pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
pub const CLOCK_FREQ: usize = 1250_0000; //freq for qemu -m virt

//这些常量是临时的，仅在相当于rcore-ch3的版本使用
pub const KERNEL_STACK_SIZE: usize = 0x4_0000; // 256 KB, per CPU

pub const MAX_APP_NUM: usize = 20; // 应用程序个数限制
pub const APP_BASE_ADDRESS: usize = 0x8040_0000;
pub const APP_SIZE_LIMIT: usize = 0x2_0000;

//pub const MEMORY_END: usize = 0x80800000; //128MB
pub const PAGE_SIZE: usize = 0x1000; // 4 KB
pub const PAGE_SIZE_BITS: usize = 0xc; // 4 KB = 2^12
pub const EMPTY_TASK: usize = usize::MAX;

pub const USER_STACK_SIZE: usize = 0x10_0000; // 1 MB
pub const USER_STACK_OFFSET: usize = 0x4000_0000 - USER_STACK_SIZE;
pub const USER_VIRT_ADDR_LIMIT: usize = 0xFFFF_FFFF;

pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
pub const PHYS_MEMORY_OFFSET: usize = 0x8000_0000;
pub const PHYS_MEMORY_END: usize = 0x8800_0000;

pub const DEVICE_START: usize = 0x9000_0000;
pub const DEVICE_END: usize = 0x9800_0000;
