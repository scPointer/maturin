//! 提供一个跳板接口，用于让内核外的其它模块调用 Task 相关接口。

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use base_file::File;
use spin::Once;

/// 这个接口定义了一些可供外部模块调用的 Task 接口。
pub trait TaskTrampoline: Sync {
    fn suspend_current_task(&self);
    fn get_file(&self, fd: usize) -> Option<Arc<dyn File>>;
}

static TASK: Once<&'static dyn TaskTrampoline> = Once::new();

/// 内核需要调用该方法，传入内核函数的引用，来初始化该跳板模块。
pub fn init_task_trampoline(task: &'static dyn TaskTrampoline) {
    TASK.call_once(|| task);
}

/// 调用内核的 `suspend_current_task` 方法。
pub fn suspend_current_task() {
    TASK.get().unwrap().suspend_current_task();
}

/// 从当前任务的文件描述符中找到指定文件。
pub fn get_file(fd: usize) -> Option<Arc<dyn File>> {
    TASK.get().unwrap().get_file(fd)
}
