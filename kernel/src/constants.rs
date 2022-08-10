//! OS运行时用到的常量

//#![deny(missing_docs)]

#![allow(dead_code)]
/// 是否是 sifive 平台
pub const PLATFORM_SIFIVE: bool = false;
/// 编号最小的可用的 cpu_id
/// - virt 下，每个核都可用，所以是从0开始
/// - sifive 下，0号是小核，目前还用不到，所以从1开始
pub const FIRST_CPU_ID: usize = if PLATFORM_SIFIVE { 1 } else { 0 };
/// 指定一个特定的 cpu，用于执行启动过程中只能进行一次的初始化过程
//pub const BOOTSTRAP_CPU_ID: usize = FIRST_CPU_ID;
/// 最大的cpu_id再+1，可以认为是总的核数(无论是否使用)。目前在 virt 下是 4，在 sifive 下是 5
pub const CPU_ID_LIMIT: usize = FIRST_CPU_ID + 4;
/// 最后一个 CPU 的编号
pub const LAST_CPU_ID: usize = CPU_ID_LIMIT - 1;
/// 时钟频率，和平台有关
pub const CLOCK_FREQ: usize = if PLATFORM_SIFIVE { 100_0000 } else { 1250_0000 };
/// 是否单核运行。单核运行时，则其他核只启动，不运行用户程序
pub const IS_SINGLE_CORE: bool = true;
/// 是否在启动后暂停。如果为 true，则所有核都只启动，不进入用户程序
pub const SPIN_LOOP_AFTER_BOOT: bool = false;
/// 运行时是否打印基本的信息
pub const BASE_INFO: bool = false;
/// 页表中每页的大小
pub const PAGE_SIZE: usize = 0x1000; // 4 KB
/// 即 log2(PAGE_SIZE)
pub const PAGE_SIZE_BITS: usize = 0xc; // 4 KB = 2^12
/// 内核栈大小
pub const KERNEL_STACK_SIZE: usize = 0x8_000; // 1 MB
/// 内核堆的大小
pub const KERNEL_HEAP_SIZE: usize = 0x40_0000; // 4 MB
/// 用户栈大小
pub const USER_STACK_SIZE: usize = 0x2_0000; // 128 KB,
/// 初始用户栈大小，用于存放 argc/argv/envs/auxv
pub const USER_INIT_STACK_SIZE: usize = 0x4000; // 16 KB,
/// 用户栈底位置。同时也是最开始的用户堆顶位置
pub const USER_STACK_OFFSET: usize = 0x4000_0000 - USER_STACK_SIZE;
/// 用户地址最大不能超过这个值
pub const USER_VIRT_ADDR_LIMIT: usize = 0xFFFF_FFFF;
/// 内核中虚拟地址相对于物理地址的偏移
pub const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;
/// 表示内存的地址段由此开始
pub const PHYS_MEMORY_OFFSET: usize = 0x8000_0000;
/// 表示内存的地址段到此为止
pub const PHYS_MEMORY_END: usize = 0x8800_0000;

/// 入口用户程序。OS启动后，只会启动以此为名的用户程序。
/// 一般来说，这个程序会通过 fork / exec 启动终端和其他程序
pub const ORIGIN_USER_PROC_NAME: &str = "start";

/// 最小的 tid(进程号) 是 0，最大的 pid 是 TID_LIMIT-1
pub const TID_LIMIT: usize = 4096;
/// 预设的文件描述符数量限制
pub const FD_LIMIT_ORIGIN: usize = 64;
/// 最大允许的文件描述符数量
pub const FD_LIMIT_HARD: usize = 256;
/// sys_pipe创建的管道的大小，单位为字节
pub const PIPE_SIZE: usize = 0x1_000;

/// 一段左闭右开的地址区间，.0 为左端点， .1 为右端点，
pub struct AddrArea(pub usize, pub usize);
/// 用于设备 MMIO 的内存段。这些地址会在页表中做恒等映射
pub const MMIO_REGIONS: &[AddrArea] = &[AddrArea(0x10001000, 0x10002000)];

/// 是否是比赛评测。线上评测时要求OS像一个批处理系统一样工作，这可能导致内核不会直接去拿初始进程并运行
pub const IS_TEST_ENV: bool = true;
/// 测试环境下，文件系统镜像是否是由qemu引入
pub const IS_PRELOADED_FS_IMG: bool = false;
/// 文件系统镜像的大小。注意这个量和 fs-init 模块中 `/src/main.rs` 里生成镜像时的大小相同。
/// 启动时会从 .data 段加载加载
const LOCAL_FS_IMG_SIZE: usize = 16 * 2048 * 512; // 16MB
/// 测试时的文件系统镜像大小。
/// 注意因为这个文件太大，默认是已经被qemu加载好了，启动时不会加载
const TEST_FS_IMG_SIZE: usize = 0x4000_0000; // 1GB
/// 文件系统镜像大小。只有这个常量可以被其他文件使用，而上面两个不能
pub const FS_IMG_SIZE: usize = if IS_PRELOADED_FS_IMG {
    TEST_FS_IMG_SIZE
} else {
    LOCAL_FS_IMG_SIZE
};
/// 设备(sdcard)映射到内存的起始位置
pub const DEVICE_START: usize = 0x9000_0000;
/// 设备映射到内存的最后位置
pub const DEVICE_END: usize = DEVICE_START + FS_IMG_SIZE;

/// 文件系统的根目录，注意斜杠方向
pub const ROOT_DIR: &str = "./";
/// sys_open 时的参数，表示在当前目录下
pub const AT_FDCWD: i32 = -100;
/// 无父进程
pub const NO_PARENT: usize = usize::MAX;
/// 临时文件的大小限制
pub const TMP_SIZE_LIMIT: usize = 0x8_000; // 1 MB

/// 限制 mmap 的最长长度
pub const MMAP_LEN_LIMIT: usize = 0x100_0000; // 16 MB
/// 如果 elf 的 phdr 指示 base 是 0(如 libc-test 的 libc.so)，则需要找一个非0的位置放置
pub const ELF_BASE_RELOCATE: usize = 0x400_0000;

/// signal 中用到的 bitset 长度。
pub const SIGSET_SIZE_IN_BYTE: usize = 8;
/// 所有可能的信号数。有多少可能的信号，内核就要为其保存多少个 SigAction
pub const SIGSET_SIZE_IN_BIT: usize = SIGSET_SIZE_IN_BYTE * 8; // =64
/// SIGINFO 要求把一些信息存在用户栈上，从用户栈开辟一块空间来保存它们
pub const USER_STACK_RED_ZONE: usize = 0x200; // 512 B
/// sys_sendfile64 中使用 buffer 的大小
pub const SENDFILE_BUFFER_SIZE: usize = 0x2000;
