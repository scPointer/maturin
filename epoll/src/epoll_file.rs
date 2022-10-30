//! epoll 类型文件

use base_file::File;
use lock::Mutex;
use alloc::sync::Arc;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use syscall::ErrorNo;
use task_trampoline::{get_file, suspend_current_task};
use crate::{EpollEvent, EpollCtl, EpollEventType};

/// 用作 epoll 的文件
pub struct EpollFile {
    inner: Arc<Mutex<EpollFileInner>>,
}

/// epoll 内部可变部分
struct EpollFileInner {
    /// 监控的所有文件(fd)。key 不用 Arc<dyn File> 只是因为不好针对 map 做
    interest_list: BTreeMap<i32, EpollEvent>,
    /// 已经相应事件的文件(fd)
    _ready_list: BTreeSet<i32>,
}

/// epoll 用到的选项，输入一个要求监控的事件集(events)，返回一个实际发生的事件集(request events)
fn poll(file: Arc<dyn File>, events: EpollEventType) -> EpollEventType {
    let mut ret = EpollEventType::empty();
    if file.in_exceptional_conditions() {
        ret |= EpollEventType::EPOLLERR;
    }
    if file.is_hang_up() {
        ret |= EpollEventType::EPOLLHUP;
    }
    if events.contains(EpollEventType::EPOLLIN) && file.ready_to_read() {
        ret |= EpollEventType::EPOLLIN;
    }
    if events.contains(EpollEventType::EPOLLOUT) && file.ready_to_write() {
        ret |= EpollEventType::EPOLLOUT;
    }
    ret
}


impl EpollFile {
    /// 新建一个 epoll 文件
    pub fn new() -> Self {
        Self {
            inner : Arc::new(Mutex::new(EpollFileInner {
                interest_list: BTreeMap::new(),
                _ready_list: BTreeSet::new(),
            }))
        }
    }
    /// 获取另一份 epoll 文件。即使这个文件被删除，只要 fd_manager 里还存有一份，
    /// 内部 inner 上的 Arc 就不会不归零，数据就不会被删除
    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone()
        }
    }
    /// 进行控制操作，如成功则返回 Ok(())，否则返回对应的错误编号
    pub fn epoll_ctl(&self, op: EpollCtl, fd: i32, event: EpollEvent) -> Result<(), ErrorNo> {
        let list = &mut self.inner.lock().interest_list;
        match op {
            EpollCtl::ADD => {
                if list.contains_key(&fd) {
                    return Err(ErrorNo::EEXIST);
                } else {
                    list.insert(fd, event);
                }
            },
            EpollCtl::MOD => {
                if list.contains_key(&fd) {
                    // 根据 BTreeMap 的语义，这里的 insert 相当于把原来的值替换掉
                    list.insert(fd, event);
                } else {
                    return Err(ErrorNo::ENOENT);
                }
            },
            EpollCtl::DEL => {
                if list.remove(&fd).is_none() {
                    return Err(ErrorNo::ENOENT);
                }
            }
        }
        Ok(())
    }
    /// 获取 interest_list 中的所有 epoll 事件
    pub fn get_epoll_events(&self) -> Vec<EpollEvent> {
        let interest = &self.inner.lock().interest_list;
        let mut events: Vec<EpollEvent> = Vec::new();
        for (fd, evt) in interest {
            let mut nevt = *evt;
            if *fd as u64 != nevt.data {
                // warn!("fd: {} is not in Event: {:?}", fd, evt);
                nevt.data = *fd as u64;
            }
            events.push(nevt);
        }
        return events;
    }

    /// 实现 epoll_wait 系统调用，返回的第一个参数 0 表示超时，正数表示响应的事件个数，第二个参数表示响应后的 `epoll_events`
    pub fn epoll_wait(&self, expire_time: usize) -> Vec<EpollEvent> {
        let epoll_events = self.get_epoll_events();
        let mut ret_events: Vec<EpollEvent> = Vec::new();
        loop {
            // 已触发的 fd
            for req_fd in &epoll_events {
                if let Some(file) = get_file(req_fd.data as usize) {
                    let revents = poll(file, req_fd.events);
                    if !revents.is_empty() {
                        ret_events.push(EpollEvent {
                            events: revents,
                            data: req_fd.data,
                        });
                    }
                } else {
                    ret_events.push(EpollEvent {
                        events: EpollEventType::EPOLLERR,
                        data: req_fd.data,
                    });
                }
            }
            if !ret_events.is_empty() {
                // 正常返回响应了事件的fd个数
                return ret_events;
            }
            // 否则暂时 block 住
            if timer::get_time_ms() > expire_time {
                // 超时返回0
                return ret_events;
            }
            suspend_current_task();
        }
    }
}

impl File for EpollFile {
    /// epoll 文件不可直接读
    fn read(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
    /// epoll 文件不可直接写
    fn write(&self, _buf: &[u8]) -> Option<usize> {
        None
    }
}
