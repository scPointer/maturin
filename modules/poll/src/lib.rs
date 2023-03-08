//! 实现了 sys_ppoll 的系统调用
//!
//! **该模块依赖 `task-trampoline`，因此使用该模块前，请先按照 `task-trampoline` 的文档说明进行初始化。**

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem::size_of;
use base_file::File;
use bitflags::bitflags;
use task_trampoline::{get_file, manually_alloc_type, manually_alloc_user_str, suspend_current_task};

bitflags! {
    /// sys_ppoll 使用，表示对应在文件上等待或者发生过的事件
    pub struct PollEvents: u16 {
        /// 可读
        const IN = 0x0001;
        /// 可写
        const OUT = 0x0004;
        /// 报错
        const ERR = 0x0008;
        /// 已终止，如 pipe 的另一端已关闭连接的情况
        const HUP = 0x0010;
        /// 无效的 fd
        const INVAL = 0x0020;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// sys_ppoll 系统调用参数用到的结构
pub struct PollFd {
    /// 等待的 fd
    pub fd: i32,
    /// 等待的事件
    pub events: PollEvents,
    /// 返回的事件
    pub revents: PollEvents,
}

/// poll / ppoll 用到的选项，输入一个要求监控的事件集(events)，返回一个实际发生的事件集(request events)
fn poll(file: Arc<dyn File>, events: PollEvents) -> PollEvents {
    let mut ret = PollEvents::empty();
    if file.in_exceptional_conditions() {
        ret |= PollEvents::ERR;
    }
    if file.is_hang_up() {
        ret |= PollEvents::HUP;
    }
    if events.contains(PollEvents::IN) && file.ready_to_read() {
        ret |= PollEvents::IN;
    }
    if events.contains(PollEvents::OUT) && file.ready_to_write() {
        ret |= PollEvents::OUT;
    }
    ret
}

/// ppoll 系统调用的具体实现
///
/// # Arguments
///
/// * `fds`: 一个 `PollFd` 的列表。
/// * `expire_time`: 超时的时间戳，会与 `get_time()` 接口返回的时间戳比较。
///
/// returns: (usize, Vec<PollFd>) 第一个参数遵守 ppoll 系统调用的返回值约定，第二个参数为返回的 `PollFd` 列表
fn ppoll(mut fds: Vec<PollFd>, expire_time: usize) -> (usize, Vec<PollFd>) {
    loop {
        // 已触发的 fd
        let mut set: usize = 0;
        for req_fd in &mut fds {
            if let Some(file) = get_file(req_fd.fd as usize) {
                req_fd.revents = poll(file, req_fd.events);
                if !req_fd.revents.is_empty() {
                    set += 1;
                }
            } else {
                req_fd.revents = PollEvents::ERR;
                set += 1;
            }
        }
        if set > 0 {
            // 如果找到满足条件的 fd，则返回找到的 fd 数量
            return (set, fds);
        }
        // 否则暂时 block 住
        if timer::get_time() > expire_time {
            return (0, fds);
        }
        suspend_current_task();
    }
}

/// 实现 ppoll 的系统调用
pub fn sys_ppoll(
    ufds: *mut PollFd,
    nfds: usize,
    timeout: *const timer::TimeSpec, // ppoll 不会更新 timeout 的值，而 poll 会
    _sigmask: *const usize
) -> Result<usize, syscall::ErrorNo> {
    // TODO: MemorySet 模块化之后，是否可以不再依赖 task-trampoline 的 manually_alloc_user_str 接口
    // let task = get_current_task().unwrap();
    // let mut task_vm = task.vm.lock();
    // if task_vm.manually_alloc_user_str(ufds as *const u8, nfds * size_of::<PollFd>()).is_err() {
    //     return Err(syscall::ErrorNo::EFAULT); // 无效地址
    // }
    if manually_alloc_user_str(ufds as *const u8, nfds * size_of::<PollFd>()).is_err() {
        return Err(syscall::ErrorNo::EFAULT); // 无效地址
    }
    let mut fds: Vec<PollFd> = Vec::new();
    for i in 0..nfds {
        unsafe { fds.push(*ufds.add(i)); }
    }
    // 过期时间
    // 这里用**时钟周期数**来记录，足够精确的同时 usize 也能存下。实际用微秒或者纳秒应该也没问题。
    let expire_time = if timeout as usize != 0 {
        // if task_vm.manually_alloc_type(timeout).is_err() {
        //     return Err(syscall::ErrorNo::EFAULT); // 无效地址
        // }
        if manually_alloc_type(timeout).is_err() {
            return Err(syscall::ErrorNo::EFAULT); // 无效地址
        }
        timer::get_time() + unsafe { (*timeout).get_ticks() }
    } else {
        usize::MAX // 没有过期时间
    };
    // drop(task_vm); // select 的时间可能很长，之后不用 vm 了就及时释放
    let (result, ret_fds) = ppoll(fds, expire_time);
    for i in 0..ret_fds.len() {
        unsafe { *ufds.add(i) = ret_fds[i]; }
    }
    Ok(result)
}
