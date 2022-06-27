//! 系统调用中的选项/类型
//!
//! 实现系统调用中出现的各种由参数指定的选项和结构体

#![deny(missing_docs)]

use bitflags::*;
use core::ops::{Add};
use core::cmp::Ordering;
use core::mem::size_of;

use crate::memory::PTEFlags;

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

//pub const nsec_per_sec: usize = 1_000_000_000;
// 因为测例库说明里的 tv_nsec 实际实现是 usec，所以要把这个量当微秒实现
pub const nsec_per_sec: usize = 1_000_000;

/// sys_gettimeofday 和 sys_nanosleep 中指定的结构体类型
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TimeSpec {
    pub tv_sec: usize,
    pub tv_nsec: usize,
}

impl TimeSpec {
    /// 通过时间值创建数组。请保证 seconds 为非负数
    pub fn new(seconds: f64) -> Self {
        let tv_sec = seconds as usize;
        let left = seconds - tv_sec as f64;
        Self {
            tv_sec: tv_sec,
            tv_nsec: (left * nsec_per_sec as f64) as usize,
        }
    }
    /// 返回以秒为单位的时间
    pub fn time_in_sec(&self) -> f64 {
        self.tv_sec as f64 + self.tv_nsec as f64 / 1_000_000_000 as f64
    }
}

impl Add for TimeSpec {
    type Output = TimeSpec;
    fn add(self, other: Self) -> Self {
        let mut new_ts = Self {
            tv_sec: self.tv_sec + other.tv_sec,
            tv_nsec: self.tv_nsec + other.tv_nsec,
        };
        if new_ts.tv_nsec >= nsec_per_sec {
            new_ts.tv_sec += 1;
            new_ts.tv_nsec -= nsec_per_sec;
        }
        new_ts
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
            release: Self::from_str("0.1"),
            version: Self::from_str("0.1"),
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
    pub base: *const u8,
    pub len: usize,
}
