//! 和 RISC-V 时间处理相关的方法

#![no_std]

use core::ops::Add;
use riscv::register::time;
use syscall::ErrorNo;
use task_trampoline::{manually_alloc_type, raw_time, raw_timer, set_timer, suspend_current_task};

/* Constants */

/// 时钟频率，和平台有关
pub const CLOCK_FREQ: usize = if cfg!(feature = "sifive") { 100_0000 } else { 1250_0000 };
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

// sys_getrusage 用到的选项
/// 获取当前进程的资源统计
pub const RUSAGE_SELF: i32 = 0;
/// 获取当前进程的所有 **已结束并等待资源回收的** 子进程资源统计
pub const RUSAGE_CHILDREN: i32 = -1;
/// 获取当前线程的资源统计
pub const RUSAGE_THREAD: i32 = 1;

/* Methods */

/// 读 mtime 计时器的值
pub fn get_time() -> usize {
    time::read()
}

/// 获取毫秒格式的时间值。注意这不一定代表进程经过的时间值
pub fn get_time_ms() -> usize {
    (time::read() * 1000) / CLOCK_FREQ
}

pub fn get_time_sec() -> usize {
    time::read() / CLOCK_FREQ
}

pub fn get_time_us() -> usize {
    time::read() / MACHINE_TICKS_PER_USEC
}

/// 当前时间为多少秒(浮点数格式)
pub fn get_time_f64() -> f64 {
    get_time() as f64 / CLOCK_FREQ as f64
}

/// 获取下一次中断时间
pub fn get_next_trigger() -> u64 {
    (get_time() + CLOCK_FREQ / INTERRUPT_PER_SEC).try_into().unwrap()
}

/* Structs */

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

/// sys_gettimer / sys_settimer 指定的类型，用户输入输出计时器
pub struct ITimerVal {
    it_interval: TimeVal,
    it_value: TimeVal,
}

/* Syscall */

/// 获取系统时间并存放在参数提供的数组里
pub fn sys_get_time_of_day(time_val: *mut TimeVal) -> Result<usize, ErrorNo> {
    //info!("sys_gettimeofday at {:x}", time_val as usize);
    if manually_alloc_type(time_val).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    unsafe {
        (*time_val) = TimeVal::now();
        //info!("sec = {}, usec = {}", (*time_val).sec, (*time_val).usec);
    }
    Ok(0)
}

pub fn sys_clock_gettime(_clockid: usize, time_spec: *mut TimeSpec) -> Result<usize, ErrorNo> {
    //info!("sys_clock_gettime clock id = {clockid} at {:x}", time_spec as usize);
    if manually_alloc_type(time_spec).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    unsafe {
        (*time_spec) = TimeSpec::now();
        //info!("sec = {}, nsec = {}", (*time_spec).tv_sec, (*time_spec).tv_nsec);
    }
    Ok(0)
}

/// 该进程休眠一段时间
pub fn sys_nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> Result<usize, ErrorNo> {
    let end_time = unsafe { get_time_f64() + (*req).time_in_sec() };
    //let now = get_time_f64();
    //info!("now {} end time {}", now, end_time);
    while get_time_f64() < end_time {
        suspend_current_task();
    }
    // 如果用户提供了 rem 数组，则需要修改它
    if rem as usize != 0 {
        unsafe {
            (*rem) = TimeSpec::new(0.0);
        }
    }
    Ok(0)
}

/// 将进程的运行时间信息传入用户提供的数组。详见 TMS 类型声明
pub fn sys_times(tms_ptr: *mut TMS) -> Result<usize, ErrorNo> {
    let (utime, stime) = raw_time();
    //info!("times: get utime {utime}ms, stime {stime}ms");
    unsafe {
        (*tms_ptr).tms_utime = utime;
        (*tms_ptr).tms_stime = stime;
        (*tms_ptr).tms_cutime = utime;
        (*tms_ptr).tms_cstime = stime;
    }
    Ok(get_time_us() / USEC_PER_INTERRUPT)
}

pub fn sys_getrusage(who: i32, utime: *mut TimeVal) -> Result<usize, ErrorNo> {
    let stime = unsafe { utime.add(1) };
    if manually_alloc_type(utime).is_err() || manually_alloc_type(stime).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    match who {
        RUSAGE_SELF | RUSAGE_CHILDREN | RUSAGE_THREAD => {
            // todo: 目前对于所有的 who 都只统计了当前任务，其实应该细化
            let (utime_us, stime_us) = raw_time();
            unsafe {
                *utime = utime_us.into();
                *stime = stime_us.into();
            }
            //unsafe {*utime = get_time_us().into(); *stime = get_time_us().into();}
            //unsafe { if task.get_tid_num() == 4  {*utime = (get_time_us() * 10).into();} }
            //println!("utime {}",  get_time_us());
            //let (utime, stime) = task_time.output_raw();
            //println!("tid {} who {who} getrusage: utime {utime}us, stime {stime}us", task.get_tid_num());
            Ok(0)
        }
        _ => Err(ErrorNo::EINVAL),
    }
}

pub fn sys_gettimer(_which: usize, curr_value: *mut ITimerVal) -> Result<usize, ErrorNo> {
    if manually_alloc_type(curr_value).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    let (timer_interval_us, timer_remained_us) = raw_timer();
    unsafe {
        (*curr_value).it_interval = timer_interval_us.into();
        (*curr_value).it_value = timer_remained_us.into();
    };
    Ok(0)
}

pub fn sys_settimer(
    which: usize,
    new_value: *const ITimerVal,
    old_value: *mut ITimerVal,
) -> Result<usize, ErrorNo> {
    if manually_alloc_type(new_value).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    if old_value as usize != 0 {
        // 需要返回旧值
        if manually_alloc_type(old_value).is_err() {
            return Err(ErrorNo::EFAULT);
        }
        let (timer_interval_us, timer_remained_us) = raw_timer();
        unsafe {
            (*old_value).it_interval = timer_interval_us.into();
            (*old_value).it_value = timer_remained_us.into();
        };
    }
    let (timer_interval_us, timer_remained_us) = unsafe {
        ((*new_value).it_interval.into(), (*new_value).it_value.into())
    };
    if set_timer(timer_interval_us, timer_remained_us, which) {
        Ok(0)
    } else {
        Err(ErrorNo::EFAULT) // 设置不成功，说明参数 which 错误
    }
}