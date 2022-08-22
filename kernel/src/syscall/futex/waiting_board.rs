//! 一张全局的表，表示每个线程是否在等待被唤醒

use super::Waiter;
use alloc::boxed::Box;
use alloc::vec::Vec;
use lock::Mutex;

/// 从 tid 获取信号相关信息
static WAITING_BOARD: Mutex<Vec<Option<Box<dyn Waiter>>>> = Mutex::new(Vec::new());

/// 设置一个线程等待某个事件
/// 在切换线程进入时会检查是否触发 waiter
pub fn set_waiter_for_thread(tid: usize, waiter: Box<dyn Waiter>) {
    let mut waiting_board = WAITING_BOARD.lock();
    if tid >= waiting_board.len() {
        for _ in waiting_board.len()..=tid {
            waiting_board.push(None);
        }
    }
    waiting_board[tid] = Some(waiter);
}

/// 检查线程是否在等待某种资源
pub fn check_thread_blocked(tid: usize) -> bool {
    let mut waiting_board = WAITING_BOARD.lock();
    if tid >= waiting_board.len() {
        false
    } else {
        let waiter = waiting_board[tid].take();
        if waiter.is_none() || waiter.as_ref().unwrap().is_woken() {
            false
        } else {
            waiting_board[tid] = waiter;
            true
        }
    }
}

/// 唤醒某个线程，如 waiter 存在，则返回 true(无论是否之前就被唤醒)。
/// 注意，这不是线程被唤醒的唯一方式。如果在除了 WAITING_BOARD 之外的地方也保存了对应的 Arc<dyn Waiter>
/// 那么 waiter 也可能在其他地方被设置为 woken
pub fn wake_thread(tid: usize) -> bool {
    let mut waiting_board = WAITING_BOARD.lock();
    if tid >= waiting_board.len() {
        false
    } else {
        waiting_board[tid].as_mut().map(|w| w.wake()).is_some()
    }
}
