//! 处理 pselect 相关的结构

use alloc::sync::Arc;
use alloc::vec::Vec;
use base_file::File;
use core::mem::size_of;
use epoll::{EpollCtl, EpollErrorNo, EpollEvent, EpollEventType, EpollFile};
use lock::MutexGuard;

use crate::constants::FD_LIMIT_HARD;
use crate::file::{FdManager, PollEvents};
use crate::memory::MemorySet;
use crate::signal::ShadowBitset;
use crate::task::{get_current_task, suspend_current_task};
use crate::timer::{get_time, get_time_ms, TimeSpec};

use super::{ErrorNo, PollFd, SysResult};

/// 获取 fd 指向文件的集合，
/// 每个文件存在 arc 里，每个 fd 值存在一个 usize 里，然后在用户地址原地清空建立一个 ShadowBitset。
///
/// 如果失败，如用户地址不合法 / fd 不存在，则返回对应错误
///
/// 这样做是因为，select / pselect 处理的 bitset 不长，也没有范围操作，但需要频繁读写，
/// 此时存在 vec 里反而比存在 bitset 里容易
fn init_fd_sets(
    addr: *mut usize,
    len: usize,
    vm: &mut MutexGuard<MemorySet>,
    fd_manager: &MutexGuard<FdManager>,
) -> Result<(Vec<Arc<dyn File>>, Vec<usize>, ShadowBitset), ErrorNo> {
    let shadow_bitset = unsafe { ShadowBitset::from_addr(addr, len) };
    if addr as usize == 0 {
        // 检查输入地址，如果为空则这个集合为空
        return Ok((Vec::new(), Vec::new(), shadow_bitset));
    }
    if vm.manually_alloc_page(addr as usize).is_err() {
        // 其实还应检查 addr + ((len + 63) & 63)
        return Err(ErrorNo::EFAULT);
    }
    // 读取对应 fd
    let fds: Vec<usize> = (0..len)
        .filter(|&fd| unsafe { shadow_bitset.check(fd) })
        .collect();
    // 查找 fd 是否都对应文件
    if let Some(files) = fd_manager.get_files_if_all_exists(&fds) {
        // 清空这一段的 bitset，直到之后 select 到可读/可写/异常的文件才写入
        unsafe {
            shadow_bitset.clear();
        }
        Ok((files, fds, shadow_bitset))
    } else {
        Err(ErrorNo::EBADF)
    }
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

pub fn sys_pselect6(
    nfds: usize,
    readfds: *mut usize,
    writefds: *mut usize,
    exceptfds: *mut usize,
    timeout: *const TimeSpec, // pselect 不会更新 timeout 的值，而 select 会
    _sigmask: *const usize,
) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    if nfds >= FD_LIMIT_HARD {
        return Err(ErrorNo::EINVAL);
    }
    let (rfile, rfd, rset) = init_fd_sets(readfds, nfds, &mut task_vm, &fd_manager)?;
    let (wfile, wfd, wset) = init_fd_sets(writefds, nfds, &mut task_vm, &fd_manager)?;
    let (efile, efd, eset) = init_fd_sets(exceptfds, nfds, &mut task_vm, &fd_manager)?;
    // 过期时间
    // 注意 pselect 不会修改用户空间中的 timeout，所以需要内核自己记录
    // 这里用**时钟周期数**来记录，足够精确的同时 usize 也能存下。实际用微秒或者纳秒应该也没问题。
    let expire_time = if timeout as usize != 0 {
        if task_vm.manually_alloc_type(timeout).is_err() {
            return Err(ErrorNo::EFAULT); // 无效地址
        }
        get_time() + unsafe { (*timeout).get_ticks() }
    } else {
        usize::MAX // 没有过期时间
    };
    // 这里暂时不考虑 sigmask 的问题

    info!(
        "pselect {nfds} {:#?} {:#?} {:#?} {}(now {})",
        rfd,
        wfd,
        efd,
        expire_time,
        get_time()
    );

    drop(task_vm); // select 的时间可能很长，之后不用 vm 了就及时释放
    drop(fd_manager); // fd_manager 同理
    loop {
        // 已设置的 fd
        let mut set: usize = 0;
        if rset.is_valid() {
            // 如果设置了监视是否可读的 fd
            for i in 0..rfile.len() {
                if rfile[i].ready_to_read() {
                    unsafe {
                        rset.set(rfd[i]);
                    }
                    set += 1;
                }
            }
        }
        if wset.is_valid() {
            // 如果设置了监视是否可写的 fd
            for i in 0..wfile.len() {
                if wfile[i].ready_to_write() {
                    unsafe {
                        wset.set(wfd[i]);
                    }
                    set += 1;
                }
            }
        }
        if eset.is_valid() {
            // 如果设置了监视是否异常的 fd
            for i in 0..efile.len() {
                if efile[i].in_exceptional_conditions() {
                    unsafe {
                        eset.set(efd[i]);
                    }
                    set += 1;
                }
            }
        }
        if set > 0 {
            // 如果找到满足条件的 fd，则返回找到的 fd 数量
            return Ok(set);
        }
        // 否则暂时 block 住
        suspend_current_task();
        if get_time() > expire_time {
            // 检查超时
            return Ok(0);
        }
    }
}

pub fn sys_ppoll(
    ufds: *mut PollFd,
    nfds: usize,
    timeout: *const TimeSpec, // ppoll 不会更新 timeout 的值，而 poll 会
    _sigmask: *const usize,
) -> SysResult {
    //if nfds > 0 { return Ok(1); }
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    debug!(
        "ppoll ufds at {:x} nfds {} timeout at {:x}",
        ufds as usize, nfds, timeout as usize
    );
    if task_vm
        .manually_alloc_user_str(ufds as *const u8, nfds * size_of::<PollFd>())
        .is_err()
    {
        return Err(ErrorNo::EFAULT); // 无效地址
    }
    let mut fds: Vec<PollFd> = Vec::new();
    for i in 0..nfds {
        unsafe {
            fds.push(*ufds.add(i));
        }
    }
    // 过期时间
    // 这里用**时钟周期数**来记录，足够精确的同时 usize 也能存下。实际用微秒或者纳秒应该也没问题。
    let expire_time = if timeout as usize != 0 {
        if task_vm.manually_alloc_type(timeout).is_err() {
            return Err(ErrorNo::EFAULT); // 无效地址
        }
        get_time() + unsafe { (*timeout).get_ticks() }
    } else {
        usize::MAX // 没有过期时间
    };
    drop(task_vm); // select 的时间可能很长，之后不用 vm 了就及时释放
    loop {
        let fd_manager = task.fd_manager.lock();
        // 已触发的 fd
        let mut set: usize = 0;
        for req_fd in &mut fds {
            if let Ok(file) = fd_manager.get_file(req_fd.fd as usize) {
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
            for i in 0..fds.len() {
                unsafe {
                    *ufds.add(i) = fds[i];
                }
            }
            return Ok(set);
        }
        // 否则暂时 block 住
        if get_time() > expire_time {
            // 检查超时
            for i in 0..fds.len() {
                unsafe {
                    *ufds.add(i) = fds[i];
                }
            }
            return Ok(0);
        }
        drop(fd_manager); // fd_manager 同理
        suspend_current_task();
    }
}

/// 创建一个 epoll 文件
pub fn sys_epoll_create(_flags: usize) -> SysResult {
    info!("epoll create");
    let task = get_current_task().unwrap();
    let mut fd_manager = task.fd_manager.lock();
    let epoll_file = EpollFile::new();
    fd_manager
        .push(Arc::new(epoll_file))
        .map_err(|_| ErrorNo::EMFILE)
}

pub fn sys_epoll_ctl(epfd: i32, op: i32, fd: i32, event: *const EpollEvent) -> SysResult {
    info!("epoll ctl: epfd {epfd} op {op} fd {fd}");
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    let event = if task_vm.manually_alloc_type(event).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    } else {
        unsafe { *event }
    };
    let operator = EpollCtl::try_from(op).map_err(|_| ErrorNo::EINVAL)?; // 操作符不合法
    if let Ok(file) = fd_manager.get_file(epfd as usize) {
        return if let Some(epoll_file) = file.as_any().downcast_ref::<EpollFile>() {
            if fd_manager.get_file(fd as usize).is_err() {
                return Err(ErrorNo::EBADF); // 错误的文件描述符
            }
            epoll_file
                .epoll_ctl(operator, fd, event)
                .map(|_| 0)
                .map_err(|e| match e {
                    EpollErrorNo::EEXIST => ErrorNo::EEXIST,
                    EpollErrorNo::ENOENT => ErrorNo::ENOENT,
                })
        } else {
            Err(ErrorNo::EBADF) // 错误的文件描述符
        };
    }
    Err(ErrorNo::EBADF) // 错误的文件描述符
}

pub fn sys_epoll_wait(
    epfd: i32,
    event: *mut EpollEvent,
    maxevents: i32,
    timeout: i32,
) -> SysResult {
    info!("epoll wait: epfd {epfd} event {event:?} maxevents {maxevents} timeout {timeout}");
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_type(event).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    };
    let fd_manager = task.fd_manager.lock();
    let epoll_file = if let Ok(file) = fd_manager.get_file(epfd as usize) {
        if let Some(epoll_file) = file.as_any().downcast_ref::<EpollFile>() {
            epoll_file.clone()
        } else {
            return Err(ErrorNo::EBADF); // 错误的文件描述符
        }
    } else {
        return Err(ErrorNo::EBADF); // 错误的文件描述符
    };

    //类似poll
    let epolls = epoll_file.get_epoll_events();
    let expire_time = if timeout >= 0 {
        get_time_ms() + timeout as usize
    } else {
        usize::MAX // 没有过期时间
    };
    drop(fd_manager);
    drop(task_vm); // select 的时间可能很长，之后不用 vm 了就及时释放

    loop {
        let fd_manager = task.fd_manager.lock();
        // 已触发的 fd
        let mut set: usize = 0;
        for req_fd in &epolls {
            if let Ok(file) = fd_manager.get_file(req_fd.data as usize) {
                let revents = poll(
                    file,
                    PollEvents::from_bits_truncate(req_fd.events.bits() as u16),
                );
                if !revents.is_empty() {
                    info!("Epoll found fd {} revent {:?}", req_fd.data, revents);
                    // 回写epollevent,
                    unsafe {
                        *event.add(set) = *req_fd;
                        (*event.add(set)).events =
                            EpollEventType::from_bits_truncate(revents.bits() as u32);
                    }
                    set += 1;
                }
            } else {
                warn!("epoll can not get fd: {}", req_fd.data);
                //let revents = PollEvents::ERR;
                unsafe {
                    *event.add(set) = *req_fd;
                    (*event.add(set)).events = EpollEventType::EPOLLERR;
                }
                set += 1;
            }
        }
        if set > 0 {
            unsafe {
                info!("Epoll ret: {:?}", *event);
            }
            // 正常返回响应了事件的fd个数
            return Ok(set);
        }
        // 否则暂时 block 住
        if get_time_ms() > expire_time {
            // 超时返回0
            return Ok(0);
        }
        drop(fd_manager); // fd_manager 同理
        suspend_current_task();
    }
}
