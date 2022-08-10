//! 统计进程的用户态和内核态时间
//! 
//! 统计的时间以毫秒为单位，用于 sys_getrusage 和 sys_times
//! 
//! 目前这个模块的逻辑如下：
//! - 在 `cpu_local.rs: run_tasks()` 中切换进入/切出用户程序上下文处，开始/停止统计内核态时间
//! - 在 `trap/mod.rs: trap_handler()` 中进入/退出异常中断处理时，开始/停止统计内核态时间，停止/开始统计用户态时间
//! 
//! > 如果在 trap 的过程中，通过其他方式退出了进程，那么内核时间统计会在 `run_tasks()` 切出时中断。
//! > 这样统计的时间仍然是对的

use crate::timer::{TimeVal, get_time_us};
use crate::signal::{SignalNo, send_signal};

/// 进程的时间统计，基于 lmbench 需要，主要用于 sys_getrusage
pub struct TimeStat {
    /// 线程 id。因为到时间时会自动发送信号，所以最好另外存一份pid在这
    tid: usize,
    /// 用户态经过时间
    utime_us: usize,
    /// 内核态经过时间
    stime_us: usize,
    /// 进入用户态时标记当前系统时间，退出时累加统计
    user_tick: usize,
    /// 进入内核态时标记当前系统时间，退出时累加统计
    kernel_tick: usize,
    /// 开始运行时的系统时间
    start_tick: usize,
    /// 计时器类型
    timer_type: TimerType,
    /// 设置下一次触发计时器的区间
    /// 
    /// 当 timer_remained_us 归零时，**如果 timer_interval_us 非零 **，则将其重置为 timer_interval_us 的值；
    /// 否则，则这个计时器不再触发
    timer_interval_us: usize,
    /// 当前计时器还剩下多少时间。
    /// 
    /// 根据 timer_type 的规则不断减少，当归零时触发信号
    timer_remained_us: usize,
}

numeric_enum_macro::numeric_enum! {
    #[repr(i32)]
    #[allow(non_camel_case_types)]
    #[derive(Eq, PartialEq, Debug)]
    /// sys_settimer / sys_gettimer 中设定的 which，即计时器类型
    pub enum TimerType {
        /// 表示目前没有任何计时器(不在linux规范中，是os自己规定的)
        NONE = -1,
        /// 统计系统实际运行时间
        REAL = 0,
        /// 统计用户态运行时间
        VIRTUAL = 1,
        /// 统计进程的所有用户态/内核态运行时间
        PROF = 2,
    }
}

impl From<usize> for TimerType {
    fn from(num: usize) -> Self {
        match Self::try_from(num as i32) {
            Ok(t_type) => t_type,
            Err(_) => Self::NONE,
        }
    }
}

/// sys_gettimer / sys_settimer 指定的类型，用户输入输出计时器
pub struct ITimerVal{
    it_interval: TimeVal,
    it_value: TimeVal,
}

impl TimeStat {
    /// 新线程的时间记为 0
    pub fn new(tid: usize) -> Self {
        Self {
            tid: tid,
            utime_us: 0,
            stime_us: 0,
            user_tick: 0,
            kernel_tick: 0,
            start_tick: get_time_us(),
            timer_type: TimerType::NONE,
            timer_interval_us: 0,
            timer_remained_us: 0,
        }
    }
    /// 清空使用的时间，用于
    pub fn clear(&mut self) {
        self.utime_us = 0;
        self.stime_us = 0;
        self.user_tick = 0;
        self.kernel_tick = 0;
        self.start_tick = get_time_us();
    }
    /// 统计时间：从内核进入用户态时调用
    pub fn timer_kernel_to_user(&mut self) {
        let now = get_time_us();
        let delta = now - self.kernel_tick;
        if self.timer_type == TimerType::REAL || self.timer_type == TimerType::PROF {
            self.update_timer_and_send_signal(delta);
        }
        self.stime_us += delta;
        self.user_tick = now;
    }
    /// 统计时间：从用户进入内核态时调用
    pub fn timer_user_to_kernel(&mut self) {
        let now = get_time_us();
        let delta = now - self.kernel_tick;
        if self.timer_type != TimerType::NONE {
            self.update_timer_and_send_signal(delta);
        }
        self.utime_us += delta;
        self.kernel_tick = now;
    }
    /// 统计时间：(内核态)切换进入当前任务
    pub fn switch_into_task(&mut self) {
        self.kernel_tick = get_time_us();
    }
    /// 统计时间：(内核态)切出进入当前任务
    pub fn switch_out_task(&mut self) {
        let delta = get_time_us() - self.kernel_tick;
        self.stime_us += delta;
        if self.timer_type == TimerType::REAL || self.timer_type == TimerType::PROF {
            self.update_timer_and_send_signal(delta);
        }
    }
    /// 以 TimeVal 形式输出统计的用户态和内核态时间
    pub fn output(&self, utime: &mut TimeVal, stime: &mut TimeVal) {
        *utime = self.utime_us.into();
        *stime = self.stime_us.into();
    }
    /// 输出微秒形式的时间统计，用于调试
    pub fn output_raw(&self) -> (usize, usize) {
        (self.utime_us, self.stime_us)
    }

    /// 以 TimeVal 形式输出计时器信息
    pub fn get_timer(&self, itimer: &mut ITimerVal) {
        itimer.it_interval = self.timer_interval_us.into();
        itimer.it_value = self.timer_remained_us.into();
    }
    /// 以 TimeVal 形式读入计时器信息，返回是否设置成功(类型参数对就算设置成功)
    pub fn set_timer(&mut self, itimer: &ITimerVal, timer_type: usize) -> bool {
        self.timer_type = timer_type.into();
        self.timer_interval_us = itimer.it_interval.into();
        self.timer_remained_us = itimer.it_value.into();
        self.timer_type != TimerType::NONE
    }
    /// 从计时器中尝试减少一段时间，如果时间归零，则发送信号
    /// (**内部需要获取对应线程的 SignalReceivers，注意死锁，注意保证发送的线程仍存在**)。
    /// 然后根据 timer_interval_us 更新寄存器
    /// 
    /// 返回是否触发信号
    pub fn update_timer_and_send_signal(&mut self, delta: usize) -> bool {
        if self.timer_remained_us == 0 { // 等于0说明没有计时器，或者 one-shot 计时器已结束
            return false;
        }
        if self.timer_remained_us > delta { // 时辰未到，减少寄存器计数
            self.timer_remained_us -= delta;
            return false;
        }
        // 到此说明计时器已经到时间了，更新计时器
        // 如果是 one-shot 计时器，则 timer_interval_us == 0，这样赋值也恰好是符合语义的
        self.timer_remained_us = self.timer_interval_us;
        match &self.timer_type {
            TimerType::REAL => send_signal(self.tid, SignalNo::SIGALRM as usize),
            TimerType::VIRTUAL => send_signal(self.tid, SignalNo::SIGVTALRM as usize),
            TimerType::PROF => send_signal(self.tid, SignalNo::SIGPROF as usize),
            _ => {},
        };
        true
    }
}
