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
pub use epoll_file::EpollFile;
pub use flags::{EpollCtl, EpollEvent, EpollEventType};
use syscall::ErrorNo;
use task_trampoline::{get_file, manually_alloc_type, push_file};

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

#[cfg(test)]
mod tests {
    use crate::{
        sys_epoll_create, sys_epoll_ctl, sys_epoll_wait, EpollCtl, EpollEvent, EpollEventType,
    };
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use base_file::File;
    use lock::Mutex;

    struct MyTaskTrampoline;

    static mut FILES: Vec<Option<Arc<dyn File>>> = Vec::new();

    impl task_trampoline::TaskTrampoline for MyTaskTrampoline {
        fn suspend_current_task(&self) {}

        fn get_file(&self, fd: usize) -> Option<Arc<dyn File>> {
            unsafe {
                if fd >= FILES.len() || FILES[fd].is_none() {
                    None
                } else {
                    Some(FILES[fd].as_ref().unwrap().clone())
                }
            }
        }

        fn push_file(&self, file: Arc<dyn File>) -> Result<usize, u64> {
            unsafe {
                let fd = FILES.len();
                FILES.push(Some(file));
                Ok(fd)
            }
        }

        fn manually_alloc_user_str(&self, buf: *const u8, len: usize) -> Result<(), u64> {
            Ok(())
        }

        fn manually_alloc_range(&self, start_vaddr: usize, end_vaddr: usize) -> Result<(), u64> {
            Ok(())
        }

        fn raw_time(&self) -> (usize, usize) {
            (0, 0)
        }

        fn raw_timer(&self) -> (usize, usize) {
            (0, 0)
        }

        fn set_timer(
            &self,
            timer_interval_us: usize,
            timer_remained_us: usize,
            timer_type: usize,
        ) -> bool {
            true
        }
    }

    struct FakeFileInner {
        ready: bool,
    }

    struct FakeFile {
        pub inner: Arc<Mutex<FakeFileInner>>,
    }

    impl FakeFile {
        fn set_ready(&self, ready: bool) {
            let inner = &mut self.inner.lock();
            inner.ready = ready;
        }
    }

    impl File for FakeFile {
        fn read(&self, buf: &mut [u8]) -> Option<usize> {
            None
        }

        fn write(&self, buf: &[u8]) -> Option<usize> {
            None
        }

        fn ready_to_read(&self) -> bool {
            self.inner.lock().ready
        }
    }

    #[test]
    fn epoll_test() {
        task_trampoline::init_task_trampoline(&MyTaskTrampoline);

        let fake_file = FakeFile {
            inner: Arc::new(Mutex::new(FakeFileInner { ready: true })),
        };
        let fake_fd = task_trampoline::push_file(Arc::new(fake_file)).unwrap();

        let epoll_fd = sys_epoll_create(1).unwrap();
        let mut e = EpollEvent {
            events: EpollEventType::EPOLLIN,
            data: 0,
        };
        assert_eq!(
            sys_epoll_ctl(epoll_fd as i32, 1, fake_fd as i32, &e).unwrap(),
            0
        );

        let fake_file = task_trampoline::get_file(fake_fd).unwrap();
        let fake_file = fake_file
            .as_any()
            .downcast_ref::<FakeFile>()
            .unwrap()
            .clone();
        fake_file.set_ready(true);

        assert_eq!(sys_epoll_wait(epoll_fd as i32, &mut e, 1, 0).unwrap(), 1);
        assert_eq!(sys_epoll_wait(epoll_fd as i32, &mut e, 1, 0).unwrap(), 1);
    }
}

#[cfg(test)]
mod timer {
    pub fn get_time_ms() -> usize {
        0
    }
}
