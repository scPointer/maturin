pub const BOOTSTRAP_CPU_ID: usize = 0;
pub const LAST_CPU_ID: usize = 3;
pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
pub const CLOCK_FREQ: usize = 12500000; //freq for qemu -m virt

//这些常量是临时的，仅在相当于rcore-ch3的版本使用
pub const KERNEL_STACK_SIZE: usize = 0x4_0000; // 256 KB
pub const USER_STACK_SIZE: usize = 0x4000; // 16 KB
pub const MAX_APP_NUM: usize = 20; // 应用程序个数
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;