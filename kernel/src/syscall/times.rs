//! 与时间处理相关的系统调用

//#![deny(missing_docs)]

use crate::task::{get_current_task, suspend_current_task};
use crate::timer::TimeSpec;
use crate::timer::{get_time, get_time_f64, MACHINE_TICKS_PER_MSEC};

use super::TMS;

/// 获取系统时间并存放在参数提供的数组里
pub fn sys_get_time_of_day(time_spec: *mut TimeSpec) -> isize {
    unsafe {
        (*time_spec) = TimeSpec::get_current();
        //println!("sec = {}, nsec = {}", (*time_spec).tv_sec, (*time_spec).tv_nsec);
    }
    0
}

/// 该进程休眠一段时间
pub fn sys_nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> isize {
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
    0
}

/// 将进程的运行时间信息传入用户提供的数组。详见 TMS 类型声明
pub fn sys_times(tms_ptr: *mut TMS) -> isize {
    let start_tick = get_current_task().unwrap().get_start_tick();
    let passed = get_time() - start_tick;
    let passed_ms = passed / MACHINE_TICKS_PER_MSEC;
    unsafe {
        (*tms_ptr).tms_utime = passed_ms;
        (*tms_ptr).tms_stime = passed_ms;
        (*tms_ptr).tms_cutime = passed_ms;
        (*tms_ptr).tms_cstime = passed_ms;
    }
    passed as isize
}
