//! 等待机制，记录线程是否等待在某个事件

use crate::timer::get_time_us;

pub trait Waiter: Send + Sync {
    /// 唤醒这个waiter
    fn wake(&mut self);
    /// 查询是否已被唤醒
    fn is_woken(&self) -> bool;
}

/// 等待的时 futex
pub struct FutexWaiter {
    /// 当系统时间超过这个时间时，记为超时，自动唤醒。以微秒计算
    timed_out_us: usize,
    woken: bool,
}

impl FutexWaiter {
    /// 输入唤醒时间，如果为 None 则表示没有唤醒时间
    pub fn new(timed_out: Option<usize>) -> Self {
        Self {
            timed_out_us: timed_out.unwrap_or(usize::MAX),
            woken: false
        }
    }
}

impl Waiter for FutexWaiter {
    fn wake(&mut self) {
        self.woken = true;
    }
    fn is_woken(&self) -> bool {
        if self.woken { // 主动唤醒
            true
        } else { // 超时唤醒
            get_time_us() >= self.timed_out_us
        }
    }
}