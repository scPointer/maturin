//! 与时间处理相关的系统调用

//#![deny(missing_docs)]

use super::{ErrorNo, RUSAGE_SELF, RUSAGE_CHILDREN, RUSAGE_THREAD};
use crate::task::ITimerVal;
use crate::task::{get_current_task, suspend_current_task};
use crate::timer::{TimeSpec, TimeVal};
use crate::timer::{get_time_us, get_time_f64, USEC_PER_INTERRUPT};

use super::{SysResult, TMS};

/// 获取系统时间并存放在参数提供的数组里
pub fn sys_get_time_of_day(time_val: *mut TimeVal) -> SysResult {
    //info!("sys_gettimeofday at {:x}", time_val as usize);
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_type(time_val).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    unsafe {
        (*time_val) = TimeVal::now();
        //info!("sec = {}, usec = {}", (*time_val).sec, (*time_val).usec);
    }
    Ok(0)
}

pub fn sys_clock_gettime(_clockid: usize, time_spec: *mut TimeSpec) -> SysResult {
    //info!("sys_clock_gettime clock id = {clockid} at {:x}", time_spec as usize);
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_type(time_spec).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    unsafe {
        (*time_spec) = TimeSpec::now();
        //info!("sec = {}, nsec = {}", (*time_spec).tv_sec, (*time_spec).tv_nsec);
    }
    Ok(0)
}

/// 该进程休眠一段时间
pub fn sys_nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> SysResult {
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
pub fn sys_times(tms_ptr: *mut TMS) -> SysResult {
    let (utime, stime) = get_current_task().unwrap().time.lock().output_raw();
    //info!("times: get utime {utime}ms, stime {stime}ms");
    unsafe {
        (*tms_ptr).tms_utime = utime;
        (*tms_ptr).tms_stime = stime;
        (*tms_ptr).tms_cutime = utime;
        (*tms_ptr).tms_cstime = stime;
    }
    Ok(get_time_us() / USEC_PER_INTERRUPT)
}


pub fn sys_getrusage(who: i32, utime:*mut TimeVal) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let task_time = task.time.lock();
    let stime = unsafe { utime.add(1) };
    if task_vm.manually_alloc_type(utime).is_err() 
    || task_vm.manually_alloc_type(stime).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    match who {
        RUSAGE_SELF | RUSAGE_CHILDREN | RUSAGE_THREAD => { 
            // todo: 目前对于所有的 who 都只统计了当前任务，其实应该细化
            unsafe { task_time.output(&mut *utime, &mut *stime) };
            //unsafe {*utime = get_time_us().into(); *stime = get_time_us().into();}
            //unsafe { if task.get_tid_num() == 4  {*utime = (get_time_us() * 10).into();} }
            //println!("utime {}",  get_time_us());
            //let (utime, stime) = task_time.output_raw();
            //println!("tid {} who {who} getrusage: utime {utime}us, stime {stime}us", task.get_tid_num());
            Ok(0)
        }
        _ => {
            Err(ErrorNo::EINVAL)
        }
    }
}

pub fn sys_gettimer(_which: usize, curr_value:*mut ITimerVal) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let task_time = task.time.lock();
    if task_vm.manually_alloc_type(curr_value).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    unsafe { task_time.get_timer(&mut *curr_value) };
    Ok(0)
}

pub fn sys_settimer(which: usize, new_value: *const ITimerVal, old_value: *mut ITimerVal) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let mut task_time = task.time.lock();
    if task_vm.manually_alloc_type(new_value).is_err() {
        return Err(ErrorNo::EFAULT);
    }
    if old_value as usize != 0 { // 需要返回旧值
        if task_vm.manually_alloc_type(old_value).is_err() {
            return Err(ErrorNo::EFAULT);
        }
        unsafe { task_time.get_timer(&mut *old_value) };
    }
    if unsafe { task_time.set_timer(&*new_value, which) } {
        Ok(0)
    } else {
        Err(ErrorNo::EFAULT) // 设置不成功，说明参数 which 错误
    }
}
