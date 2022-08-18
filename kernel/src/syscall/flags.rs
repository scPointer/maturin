//! 系统调用中的选项/类型
//!
//! 实现系统调用中出现的各种由参数指定的选项和结构体

//#![deny(missing_docs)]

use bitflags::*;
use core::mem::size_of;

use crate::file::{SyncPolicy, PollEvents};
use crate::memory::PTEFlags;
use crate::signal::SignalNo;
use crate::task::CloneFlags;

bitflags! {
    /// 指定 sys_wait4 的选项
    pub struct WaitFlags: u32 {
        /// 不挂起当前进程，直接返回
        const WNOHANG = 1 << 0;
        /// 报告已执行结束的用户进程的状态
        const WIMTRACED = 1 << 1;
        /// 报告还未结束的用户进程的状态
        const WCONTINUED = 1 << 3;
    }
}

bitflags! {
    /// 指定 mmap 的选项
    pub struct MMAPPROT: u32 {
        /// 不挂起当前进程，直接返回
        const PROT_READ = 1 << 0;
        /// 报告已执行结束的用户进程的状态
        const PROT_WRITE = 1 << 1;
        /// 报告还未结束的用户进程的状态
        const PROT_EXEC = 1 << 2;
    }
}

impl Into<PTEFlags> for MMAPPROT {
    fn into(self) -> PTEFlags {
        // 记得加 user 项，否则用户拿到后无法访问
        let mut flag = PTEFlags::USER;
        if self.contains(MMAPPROT::PROT_READ) {
            flag |= PTEFlags::READ;
        }
        if self.contains(MMAPPROT::PROT_WRITE) {
            flag |= PTEFlags::WRITE;
        }
        if self.contains(MMAPPROT::PROT_EXEC) {
            flag |= PTEFlags::EXECUTE;
        }
        flag
    }
}

impl Into<SyncPolicy> for MMAPPROT {
    fn into(self) -> SyncPolicy {
        if self.contains(MMAPPROT::PROT_READ) && self.contains(MMAPPROT::PROT_WRITE) {
            SyncPolicy::SyncReadWrite
        } else if self.contains(MMAPPROT::PROT_WRITE) {
            SyncPolicy::SyncWrite
        } else {
            // 其他情况默认为读。此时如果实际上是不可读的，那么页表+VmArea 可以直接判断出来，不需要操作到 backend 文件
            SyncPolicy::SyncRead
        }
    }
}

bitflags! {
    pub struct MMAPFlags: u32 {
        /// 对这段内存的修改是共享的
        const MAP_SHARED = 1 << 0;
        /// 对这段内存的修改是私有的
        const MAP_PRIVATE = 1 << 1;
        // 以上两种只能选其一

        /// 取消原来这段位置的映射
        const MAP_FIXED = 1 << 4;
        /// 不映射到实际文件
        const MAP_ANONYMOUS = 1 << 5;
        /// 映射时不保留空间，即可能在实际使用mmp出来的内存时内存溢出
        const MAP_NORESERVE = 1 << 14;
    }
}

// from libc (sys/mman.h)
/*
#define MAP_SHARED     0x01
#define MAP_PRIVATE    0x02
#define MAP_SHARED_VALIDATE 0x03
#define MAP_TYPE       0x0f
#define MAP_FIXED      0x10
#define MAP_ANON       0x20
#define MAP_ANONYMOUS  MAP_ANON
#define MAP_NORESERVE  0x4000
#define MAP_GROWSDOWN  0x0100
#define MAP_DENYWRITE  0x0800
#define MAP_EXECUTABLE 0x1000
#define MAP_LOCKED     0x2000
#define MAP_POPULATE   0x8000
#define MAP_NONBLOCK   0x10000
#define MAP_STACK      0x20000
#define MAP_HUGETLB    0x40000
#define MAP_SYNC       0x80000
#define MAP_FIXED_NOREPLACE 0x100000
*/

/// sys_times 中指定的结构体类型
#[repr(C)]
pub struct TMS {
    /// 进程用户态执行时间
    pub tms_utime: usize,
    /// 进程内核态执行时间
    pub tms_stime: usize,
    /// 子进程用户态执行时间和
    pub tms_cutime: usize,
    /// 子进程内核态执行时间和
    pub tms_cstime: usize,
}

bitflags! {
    pub struct UtimensatFlags: u32 {
        /// 表示更新时间时如果是指向符号链接，则仅更新符号链接本身的时间，不更新其指向文件的时间
        const SYMLINK_NOFOLLOW = 1 << 8;
    }
}

/// sys_uname 中指定的结构体类型
#[repr(C)]
pub struct UtsName {
    /// 系统名称
    pub sysname: [u8; 65],
    /// 网络上的主机名称
    pub nodename: [u8; 65],
    /// 发行编号
    pub release: [u8; 65],
    /// 版本
    pub version: [u8; 65],
    /// 硬件类型
    pub machine: [u8; 65],
    /// 域名
    pub domainname: [u8; 65],
}

impl UtsName {
    /// 默认 uname。这个结构的内容跟 os 没什么关系，所以就想写啥写啥了
    pub fn default() -> Self {
        Self {
            sysname: Self::from_str("MaturinOS"),
            nodename: Self::from_str("MaturinOS - machine[0]"),
            release: Self::from_str("233"),
            version: Self::from_str("1.0"),
            machine: Self::from_str("RISC-V 64 on SIFIVE FU740"),
            domainname: Self::from_str("https://github.com/scPointer/maturin"),
        }
    }

    fn from_str(info: &str) -> [u8; 65] {
        let mut data: [u8; 65] = [0; 65];
        data[..info.len()].copy_from_slice(info.as_bytes());
        data
    }
}

/// sys_getdents64 中指定的结构体类型
#[repr(C)]
pub struct Dirent64 {
    /// inode 编号
    pub d_ino: u64,
    /// 到下一个 Dirent64 的偏移
    pub d_off: i64,
    /// 当前 Dirent 长度
    pub d_reclen: u16,
    /// 文件类型
    pub d_type: u8,
    /// 文件名
    pub d_name: [u8; 0],
}

#[allow(unused)]
pub enum Dirent64Type {
    /// 未知类型文件
    UNKNOWN = 0,
    /// 先进先出的文件/队列
    FIFO = 1,
    /// 字符设备
    CHR = 2,
    /// 目录
    DIR = 4,
    /// 块设备
    BLK = 6,
    /// 常规文件
    REG = 8,
    /// 符号链接
    LNK = 10,
    /// socket
    SOCK = 12,
    WHT = 14,
}
impl Dirent64 {
    /// 设置一个目录项的信息
    pub fn set_info(&mut self, ino: usize, reclen: usize, d_type: Dirent64Type) {
        self.d_ino = ino as u64;
        self.d_off = -1;
        self.d_reclen = reclen as u16;
        self.d_type = d_type as u8;
    }
    /// 文件名字存的位置相对于结构体指针是多少
    pub fn d_name_offset() -> usize {
        size_of::<u64>() + size_of::<i64>() + size_of::<u16>() + size_of::<u8>()
    }
}

/// sys_writev / sys_readv 中指定的结构体类型
#[repr(C)]
pub struct IoVec {
    pub base: *mut u8,
    pub len: usize,
}

/// 错误编号
#[repr(C)]
#[derive(Debug)]
pub enum ErrorNo {
    /// 非法操作
    EPERM = -1,
    /// 找不到文件或目录
    ENOENT = -2,
    /// 找不到对应进程
    ESRCH = -3,
    /// 错误的文件描述符
    EBADF = -9,
    /// 资源暂时不可用。也可因为 futex_wait 时对应用户地址处的值与给定值不符
    EAGAIN = -11,
    /// 内存耗尽，或者没有对应的内存映射
    ENOMEM = -12,
    /// 无效地址
    EFAULT = -14,
    /// 设备或者资源被占用
    EBUSY = -16,
    /// 文件已存在
    EEXIST = -17,
    /// 不是一个目录(但要求需要是一个目录)
    ENOTDIR = -20,
    /// 是一个目录(但要求不能是)
    EISDIR = -21,
    /// 非法参数
    EINVAL = -22,
    /// fd（文件描述符）已满
    EMFILE = -24,
    /// 对文件进行了无效的 seek
    ESPIPE = -29,
    /// 超过范围。例如用户提供的buffer不够长
    ERANGE = -34,
    EPFNOSUPPORT = -96,
    EAFNOSUPPORT = -97,
}

// sys_lseek 时对应的条件
/// 从文件开头
pub const SEEK_SET: isize = 0;
/// 从当前位置
pub const SEEK_CUR: isize = 1;
/// 从文件结尾
pub const SEEK_END: isize = 2;

// sys_sigprocmask 时对应的选择
/// 和当前 mask 取并集
pub const SIG_BLOCK: i32 = 0;
/// 从当前 mask 中去除对应位
pub const SIG_UNBLOCK: i32 = 1;
/// 重新设置当前 mask
pub const SIG_SETMASK: i32 = 2;

pub fn resolve_clone_flags_and_signal(flag: usize) -> (CloneFlags, SignalNo) {
    (
        CloneFlags::from_bits(flag as u32 & (!0x3f)).unwrap(),
        SignalNo::try_from(flag as u8 & 0x3f).unwrap(),
    )
}

/// sys_prlimit64 使用的数组
#[repr(C)]
pub struct RLimit {
    /// 软上限
    pub rlim_cur: u64,
    /// 硬上限
    pub rlim_max: u64,
}

// sys_prlimit64 使用的选项
/// 用户栈大小
pub const RLIMIT_STACK: i32 = 3;
/// 可以打开的 fd 数
pub const RLIMIT_NOFILE: i32 = 7;
/// 用户地址空间的最大大小
pub const RLIMIT_AS: i32 = 9;

numeric_enum_macro::numeric_enum! {
    #[repr(usize)]
    #[allow(non_camel_case_types)]
    #[derive(Debug)]
    /// sys_fcntl64 使用的选项
    pub enum Fcntl64Cmd {
        /// 复制这个 fd，相当于 sys_dup
        F_DUPFD = 0,
        /// 获取 cloexec 信息，即 exec 成功时是否删除该 fd
        F_GETFD = 1,
        /// 设置 cloexec 信息，即 exec 成功时删除该 fd
        F_SETFD = 2,
        /// 获取 flags 信息
        F_GETFL = 3,
        /// 设置 flags 信息
        F_SETFL = 4,
        /// 复制 fd，然后设置 cloexec 信息，即 exec 成功时删除该 fd
        F_DUPFD_CLOEXEC = 1030,
    }
}

// sys_getrusage 用到的选项
/// 获取当前进程的资源统计
pub const RUSAGE_SELF: i32 = 0;
/// 获取当前进程的所有 **已结束并等待资源回收的** 子进程资源统计
pub const RUSAGE_CHILDREN: i32 = -1;
/// 获取当前线程的资源统计
pub const RUSAGE_THREAD: i32 = 1;

/// sys_sysinfo 用到的类型，详见 `https://man7.org/linux/man-pages/man2/sysinfo.2.html`
#[repr(C)]
#[derive(Debug)]
pub struct SysInfo {
    /// 启动时间(以秒计)
    pub uptime: isize,
    /// 1 / 5 / 15 分钟平均负载
    pub loads: [usize; 3],
    /// 内存总量，单位为 mem_unit Byte(见下)
    pub totalram: usize,
    /// 当前可用内存，单位为 mem_unit Byte(见下)
    pub freeram: usize,
    /// 共享内存大小，单位为 mem_unit Byte(见下)
    pub sharedram: usize,
    /// 用于缓存的内存大小，单位为 mem_unit Byte(见下)
    pub bufferram: usize,
    /// swap空间大小，即主存上用于替换内存中非活跃部分的空间大小，单位为 mem_unit Byte(见下)
    pub totalswap: usize,
    /// 可用的swap空间大小，单位为 mem_unit Byte(见下)
    pub freeswap: usize,
    /// 当前进程数，单位为 mem_unit Byte(见下)
    pub procs: u16,
    /// 高地址段的内存大小，单位为 mem_unit Byte(见下)
    pub totalhigh: usize,
    /// 可用的高地址段的内存大小，单位为 mem_unit Byte(见下)
    pub freehigh: usize,
    /// 指定 sys_info 的结构中用到的内存值的单位。
    /// 如 mem_unit = 1024, totalram = 100, 则指示总内存为 100K
    pub mem_unit: u32,
}

bitflags! {
    /// sys_renameat2 用到的选项
    pub struct RenameFlags: u32 {
        /// 不要替换目标位置的文件，如果预定位置已经有文件，不要删除它
        const NOREPLACE = 1 << 0;
        /// 交换原位置和目标位置的文件
        const EXCHANGE = 1 << 1;
        /// 替换后在原位置放一个 "whiteout" 类型对象，仅在一些文件系统中有用，这里不考虑
        const WHITEOUT = 1 << 2;
    }
}

bitflags! {
    /// sys_renameat2 用到的选项
    pub struct MSyncFlags: u32 {
        /// 可以异步做
        const ASYNC = 1 << 0;
        /// 删除同一文件的其他内存映射
        /// （这样把同一文件映射到其他位置的进程/线程可以马上得知文件被修改，然后更换新的值）
        const INVALIDATE = 1 << 1;
        /// 要求同步，即立即检查
        const SYNC = 1 << 2;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// poll 和 ppoll 用到的结构
pub struct PollFd {
    pub fd: i32, /// 等待的 fd
    pub events: PollEvents, /// 等待的事件
    pub revents: PollEvents,
}
