//! 和 RISC-V 时间处理相关的方法

use crate::{arch::set_timer, constants::CLOCK_FREQ};
use core::ops::Add;
use riscv::register::time;

/// 每秒的时钟中断数
pub const INTERRUPT_PER_SEC: usize = 10;
/// 每微秒的时钟周期数
pub const MACHINE_TICKS_PER_USEC: usize = CLOCK_FREQ / USEC_PER_SEC;
/// 每秒有多少微秒
const USEC_PER_SEC: usize = 1_000_000;
/// 每个时钟中断占多少微秒
pub const USEC_PER_INTERRUPT: usize = USEC_PER_SEC / INTERRUPT_PER_SEC;
/// 每秒的纳秒数
pub const NSEC_PER_SEC: usize = 1_000_000_000;
/// 每个时钟周期需要多少纳秒(取整)
pub const NSEC_PER_MACHINE_TICKS: usize = NSEC_PER_SEC / CLOCK_FREQ;
/// 当 nsec 为这个特殊值时，指示修改时间为现在
pub const UTIME_NOW: usize = 0x3fffffff;
/// 当 nsec 为这个特殊值时，指示不修改时间
pub const UTIME_OMIT: usize = 0x3ffffffe;

/// 读 mtime 计时器的值
pub fn get_time() -> usize {
    time::read()
}

/*
/// 获取毫秒格式的时间值。注意这不一定代表进程经过的时间值
#[allow(dead_code)]
pub fn get_time_ms() -> usize {
    time::read() / MACHINE_TICKS_PER_MSEC
}
*/

pub fn get_time_us() -> usize {
    time::read() / MACHINE_TICKS_PER_USEC
}

/// 当前时间为多少秒(浮点数格式)
pub fn get_time_f64() -> f64 {
    get_time() as f64 / CLOCK_FREQ as f64
}

/// 设置下一次时间中断
pub fn set_next_trigger() {
    set_timer(
        (get_time() + CLOCK_FREQ / INTERRUPT_PER_SEC)
            .try_into()
            .unwrap(),
    );
}

/// sys_nanosleep / sys_utimensat 中指定的结构体类型
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
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
            tv_nsec: (left * NSEC_PER_SEC as f64) as usize,
        }
    }
    /// 获取一个存有当前时间的 TimeSpec
    pub fn now() -> Self {
        Self::new(get_time_f64())
    }
    /// 返回以秒为单位的时间
    pub fn time_in_sec(&self) -> f64 {
        self.tv_sec as f64 + self.tv_nsec as f64 / NSEC_PER_SEC as f64
    }
    /// 根据 sys_utimensat 的格式修改当前结构
    pub fn set_as_utime(&mut self, other: &TimeSpec) {
        match other.tv_nsec {
            UTIME_NOW => {
                *self = TimeSpec::now();
            } // 设为当前时间
            UTIME_OMIT => {} // 忽略
            _ => {
                *self = *other;
            } // 设为指定时间
        }
    }
    /// 获取时钟周期数
    /// 考虑到 usize 有 64 位，这里应该不会溢出
    pub fn get_ticks(&self) -> usize {
        self.tv_sec * CLOCK_FREQ + self.tv_nsec / NSEC_PER_MACHINE_TICKS
    }
}

impl Add for TimeSpec {
    type Output = TimeSpec;
    fn add(self, other: Self) -> Self {
        let mut new_ts = Self {
            tv_sec: self.tv_sec + other.tv_sec,
            tv_nsec: self.tv_nsec + other.tv_nsec,
        };
        if new_ts.tv_nsec >= NSEC_PER_SEC {
            new_ts.tv_sec += 1;
            new_ts.tv_nsec -= NSEC_PER_SEC;
        }
        new_ts
    }
}

/// sys_gettimeofday / sys_rusage 中指定的类型
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    /// 当前的微秒数
    pub fn now() -> Self {
        get_time_us().into()
    }
}

impl Add for TimeVal {
    type Output = TimeVal;
    fn add(self, other: Self) -> Self {
        let mut new_ts = Self {
            sec: self.sec + other.sec,
            usec: self.usec + other.usec,
        };
        if new_ts.usec >= USEC_PER_SEC {
            new_ts.sec += 1;
            new_ts.usec -= USEC_PER_SEC;
        }
        new_ts
    }
}

impl From<TimeSpec> for TimeVal {
    fn from(spec: TimeSpec) -> Self {
        Self {
            sec: spec.tv_sec,
            usec: spec.tv_nsec / USEC_PER_SEC,
        }
    }
}

impl From<usize> for TimeVal {
    /// 输入微秒数，自动转换成秒+微秒
    fn from(usec: usize) -> Self {
        Self {
            sec: usec / USEC_PER_SEC,
            usec: usec % USEC_PER_SEC,
        }
    }
}

impl Into<usize> for TimeVal {
    /// 输入微秒数，自动转换成秒+微秒
    fn into(self) -> usize {
        self.sec * USEC_PER_SEC + self.usec
    }
}
