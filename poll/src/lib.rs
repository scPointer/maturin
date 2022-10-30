#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use base_file::File;
use bitflags::bitflags;
use task_trampoline::{get_file, suspend_current_task};

bitflags! {
    /// poll 和 ppoll 用到的选项，表示对应在文件上等待或者发生过的事件
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
/// poll 和 ppoll 用到的结构
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
pub fn ppoll(mut fds: Vec<PollFd>, expire_time: usize) -> (usize, Vec<PollFd>) {
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
