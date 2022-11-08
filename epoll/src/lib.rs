//! 实现了 epoll 相关系统调用
//!
//! **该模块依赖 `task-trampoline`，因此使用该模块前，请先按照 `task-trampoline` 的文档说明进行初始化。**
//!
//! 该模块依赖 `base-file`。如果要在内核中使用该模块，内核维护文件描述符的结构也应基于 `base-file` 实现。

#![no_std]

extern crate alloc;

mod epoll_file;
mod flags;

use alloc::sync::Arc;
pub use flags::{EpollEvent, EpollEventType, EpollCtl};
use syscall::ErrorNo;
use task_trampoline::{get_file, manually_alloc_type, push_file};
pub use epoll_file::EpollFile;

/// 执行 epoll_create 系统调用
///
/// 创建一个 epoll 文件，并通过 `task_trampoline` 添加文件的接口，将 `EpollFile` 实例添加到内核中
pub fn sys_epoll_create(_flags: usize) -> Result<usize, ErrorNo> {
    push_file(Arc::new(EpollFile::new())).map_err(|_| ErrorNo::EMFILE)
}

/// 执行 epoll_ctl 系统调用
pub fn sys_epoll_ctl(epfd: i32, op: i32, fd: i32, event: *const EpollEvent) -> Result<usize, ErrorNo> {
    let event = if manually_alloc_type(event).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    } else {
        unsafe { *event }
    };
    let operator = EpollCtl::try_from(op).map_err(|_| ErrorNo::EINVAL)?; // 操作符不合法
    if let Some(file) = get_file(epfd as usize) {
        return if let Some(epoll_file) = file.as_any().downcast_ref::<EpollFile>() {
            if get_file(fd as usize).is_none() {
                return Err(ErrorNo::EBADF); // 错误的文件描述符
            }
            epoll_file.epoll_ctl(operator, fd, event).map(|_| 0)
        } else {
            Err(ErrorNo::EBADF) // 错误的文件描述符
        }
    }
    Err(ErrorNo::EBADF) // 错误的文件描述符
}

/// 执行 epoll_wait 系统调用
pub fn sys_epoll_wait(epfd: i32, event: *mut EpollEvent, _maxevents: i32, timeout: i32) -> Result<usize, ErrorNo> {
    if manually_alloc_type(event).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    };
    let epoll_file = if let Some(file) = get_file(epfd as usize) {
        if let Some(epoll_file) = file.as_any().downcast_ref::<EpollFile>() {
            epoll_file.clone()
        } else {
            return Err(ErrorNo::EBADF) // 错误的文件描述符
        }
    } else {
        return Err(ErrorNo::EBADF) // 错误的文件描述符
    };

    //类似poll
    let expire_time = if timeout >= 0 {
        timer::get_time_ms() + timeout as usize
    } else {
        usize::MAX // 没有过期时间
    };
    let ret_events = epoll_file.epoll_wait(expire_time);
    for i in 0..ret_events.len() {
        // 回写epollevent,
        unsafe {
            (*event.add(i)).events = ret_events[i].events;
            (*event.add(i)).data = ret_events[i].data;
        }
    }
    Ok(ret_events.len())
}
