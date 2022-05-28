//! 系统调用中的选项/类型
//!
//! 实现系统调用中出现的各种由参数指定的选项和结构体

#![deny(missing_docs)]

use bitflags::*;
use core::ops::{Add};
use core::cmp::Ordering;

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
