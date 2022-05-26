#![allow(dead_code)]
pub const BOOTSTRAP_CPU_ID: usize = 0;
pub const CPU_NUM: usize =  4;
pub const LAST_CPU_ID: usize = CPU_NUM - 1;
pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
pub const CLOCK_FREQ: usize = 1250_0000; //freq for qemu -m virt
pub const IS_SINGLE_CORE: bool = true;

pub const KERNEL_STACK_SIZE: usize = 0x10_0000; // 1 MB, per CPU

//pub const MAX_APP_NUM: usize = 10; // 应用程序个数限制
//pub const APP_BASE_ADDRESS: usize = 0x8020_0000;
//pub const APP_SIZE_LIMIT: usize = 0x2_0000;
//pub const APP_ADDRESS_END: usize = APP_BASE_ADDRESS + MAX_APP_NUM * APP_SIZE_LIMIT;

pub const PAGE_SIZE: usize = 0x1000; // 4 KB
pub const PAGE_SIZE_BITS: usize = 0xc; // 4 KB = 2^12
pub const EMPTY_TASK: usize = usize::MAX;

pub const USER_STACK_SIZE: usize = 0x1_0000; // 64 KB,
pub const USER_STACK_OFFSET: usize = 0x4000_0000 - USER_STACK_SIZE;
pub const USER_VIRT_ADDR_LIMIT: usize = 0xFFFF_FFFF;

pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
pub const PHYS_MEMORY_OFFSET: usize = 0x8000_0000;
pub const PHYS_MEMORY_END: usize = 0x8800_0000;

/// 入口用户程序。OS启动后，只会启动以此为名的用户程序。
/// 一般来说，这个程序会通过 fork / exec 启动终端和其他程序
pub const ORIGIN_USER_PROC_NAME: &str = "start";

/// 最小的 pid(进程号) 是 0，最大的 pid 是 PID_LIMIT-1
pub const PID_LIMIT: usize = 4096;
/// 最大的文件描述符
pub const FD_LIMIT: usize = 256;
/// sys_pipe创建的管道的大小，单位为字节
pub const PIPE_SIZE: usize = 4000;

/// 一段左闭右开的地址区间，.0 为左端点， .1 为右端点，
pub struct AddrArea(pub usize, pub usize);
/// 用于设备 MMIO 的内存段。这些地址会在页表中做恒等映射
pub const MMIO_REGIONS: &[AddrArea] = &[AddrArea(0x10001000, 0x10002000)];

/// 是否是比赛评测。线上评测时要求OS像一个批处理系统一样工作，这可能导致内核不会直接去拿初始进程并运行
pub const IS_TEST_ENV: bool = true;
/// 测试环境下，文件系统镜像是否是由
pub const IS_PRELOADED_FS_IMG: bool = false;
/// 文件系统镜像的大小。注意这个量和 fs-init 模块中 `/src/main.rs` 里生成镜像时的大小相同。
/// 启动时会从 .data 段加载加载
const LOCAL_FS_IMG_SIZE: usize = 16 * 2048 * 512; // 16MB
/// 测试时的文件系统镜像大小。
/// 注意因为这个文件太大，默认是已经被qemu加载好了，启动时不会加载
const TEST_FS_IMG_SIZE: usize = 0x4000_0000; // 1GB
/// 文件系统镜像大小。只有这个常量可以被其他文件使用，而上面两个不能
pub const FS_IMG_SIZE: usize = if IS_PRELOADED_FS_IMG { TEST_FS_IMG_SIZE } else { LOCAL_FS_IMG_SIZE };
/// 设备(sdcard)映射到内存的起始位置
pub const DEVICE_START: usize = 0x9000_0000;
/// 设备映射到内存的最后位置
pub const DEVICE_END: usize = DEVICE_START + FS_IMG_SIZE;

/// 文件系统的根目录，注意斜杠方向
pub const ROOT_DIR: &str = "./";
/// sys_open 时的参数，表示在当前目录下
pub const AT_FDCWD: i32 = -100;