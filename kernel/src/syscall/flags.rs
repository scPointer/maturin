//! 系统调用中的选项/类型
//!
//! 实现系统调用中出现的各种由参数指定的选项和结构体

//#![deny(missing_docs)]

use bitflags::*;
use core::mem::size_of;

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
    pub d_name: *mut u8,
}

pub enum Dirent64_Type {
    UNKNOWN = 0,
    /// 先进先出的文件/队列
    FIFO = 1,
    CHR = 2,
    /// 目录
    DIR = 4,
    /// 块设备
    BLK = 6,
    /// 常规文件
    REG = 8,
    /// 符号链接
    LNK = 10,
    SOCK = 12,
    WHT = 14,
}
impl Dirent64 {
    /// 设置文件类型
    pub fn set_type(&mut self, d_type: Dirent64_Type) {
        self.d_type = d_type as u8;
    }
    /// 文件名字存的位置相对于结构体指针是多少
    pub fn d_name_offset(&self) -> usize {
        size_of::<u64>() + size_of::<i64>() +  size_of::<u16>() +  size_of::<u8>()
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
pub enum ErrorNo {
    /// 非法操作
    EPERM = -1, 
    /// 找不到文件或目录
    ENOENT = -2, 
    /// 错误的文件描述符
    EBADF = -9, 
    /// 资源暂时不可用。也可因为 futex_wait 时对应用户地址处的值与给定值不符
    EAGAIN = -11,
    /// 无效地址
    EFAULT = -14,
    /// 设备或者资源被占用
    EBUSY = -16, 
    /// 文件已存在
    EEXIST = -17,
    /// 不是一个目录
    ENOTDIR = -20,
    /// 非法参数
    EINVAL = -22, 
    /// fd（文件描述符）已满
    EMFILE = -24, 
    /// 超过范围。例如用户提供的buffer不够长
    ERANGE = -34, 
}

// sys_lseek 时对应的条件
/// 从文件开头
pub const SEEK_SET:isize = 0;
/// 从当前位置
pub const SEEK_CUR:isize = 1;
/// 从文件结尾
pub const SEEK_END:isize = 2;

// sys_sigprocmask 时对应的选择
/// 和当前 mask 取并集
pub const SIG_BLOCK:i32 = 0;
/// 从当前 mask 中去除对应位
pub const SIG_UNBLOCK:i32 = 1;
/// 重新设置当前 mask
pub const SIG_SETMASK:i32 = 2;

pub fn resolve_clone_flags_and_signal(flag: usize) -> (CloneFlags, SignalNo) {
    (
        CloneFlags::from_bits(flag as u32 & (!0x3f)).unwrap(),
        SignalNo::from(flag as u8 & 0x3f)
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
pub const RLIMIT_STACK:i32 = 3;
/// 可以打开的 fd 数
pub const RLIMIT_NOFILE:i32 = 7;
/// 用户地址空间的最大大小
pub const RLIMIT_AS:i32 = 9;
