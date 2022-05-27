//! 和 RISC-V 时间处理相关的方法

use crate::constants::CLOCK_FREQ;
use crate::arch::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 10;
const MSEC_PER_SEC: usize = 1000;
pub const MACHINE_TICKS_PER_MSEC: usize = CLOCK_FREQ / MSEC_PER_SEC;

/// 读 mtime 计时器的值
pub fn get_time() -> usize {
    time::read()
}

/// 获取毫秒格式的时间值。注意这不一定代表进程经过的时间值
pub fn get_time_ms() -> usize {
    time::read() / MACHINE_TICKS_PER_MSEC
}

/// 设置下一次时间中断
pub fn set_next_trigger() {
    set_timer((get_time() + CLOCK_FREQ / TICKS_PER_SEC).try_into().unwrap());
}
