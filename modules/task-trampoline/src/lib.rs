//! 提供一个跳板接口，用作外部模块与内核模块之间的桥梁。
//!
//! 该模块解决两个主要问题：
//! 1. 让内核外的其它模块调用内核中的接口
//! 2. 充当各种内核实现的公共规范与兼容层
//!
//! 各式各样的内核有着形形色色的实现。在模块化的大背景下，一些共性的实现逻辑可以分离为单独的模块，独立于内核存在。
//!
//! 然而，这些模块可能还需要访问内核中的一些接口，如获取某个文件描述符。这就需要一个兼容层，用于统一形形色色的内核实现。
//!
//! `task_trampoline` 就充当了这一兼容层，内核中的实现可能多种多样，但一定要实现 `TaskTrampoline` 中定义的共有接口。
//!
//! 这样，就能在多种多样的模块之间达成一个统一的规范。
//!
//! ## 如果你是其他模块的使用者，被告知需要初始化 `TaskTrampoline`
//!
//! 请在内核中定义一个自己的 `MyTaskTrampoline`，并实现我们提供的 `TaskTrampoline` trait，然后在内核启动时执行我们提供的 `init_task_trampoline` 方法。
//!
//! 例如：
//! ```rust
//! struct MyTaskTrampoline;
//!
//! impl task_trampoline::TaskTrampoline for MyTaskTrampoline {
//!     // ...
//! }
//!
//! fn main() {
//!     task_trampoline::init_task_trampoline(&MyTaskTrampoline);
//!     // ...
//! }
//! ```

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::mem::size_of;
use base_file::File;
use spin::Once;

/// 这个接口定义了一些可供外部模块调用的 Task 接口。
pub trait TaskTrampoline: Sync {
    fn suspend_current_task(&self);
    fn get_file(&self, fd: usize) -> Option<Arc<dyn File>>;
    fn push_file(&self, file: Arc<dyn File>) -> Result<usize, u64>;
    fn manually_alloc_user_str(&self, buf: *const u8, len: usize) -> Result<(), u64>;
    fn manually_alloc_range(&self, start_vaddr: usize, end_vaddr: usize) -> Result<(), u64>;
    fn raw_time(&self) -> (usize, usize);
    fn raw_timer(&self) -> (usize, usize);
    fn set_timer(&self, timer_interval_us: usize, timer_remained_us: usize, timer_type: usize) -> bool;
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

/// 插入一个新文件，返回对应的文件描述符。
pub fn push_file(file: Arc<dyn File>) -> Result<usize, u64> {
    TASK.get().unwrap().push_file(file)
}

/// 检查一段用户地址空间传来的字符串是否已分配空间，如果未分配则强制分配它
pub fn manually_alloc_user_str(buf: *const u8, len: usize) -> Result<(), u64> {
    TASK.get().unwrap().manually_alloc_user_str(buf, len)
}

/// 检查一段地址是否每一页都已分配空间，如果未分配则强制分配它
pub fn manually_alloc_range(start_vaddr: usize, end_vaddr: usize) -> Result<(), u64> {
    TASK.get().unwrap().manually_alloc_range(start_vaddr, end_vaddr)
}

/// 检查一个放在某个地址上的结构是否分配空间，如果未分配则强制分配它
pub fn manually_alloc_type<T>(user_obj: *const T) -> Result<(), u64> {
    let start_vaddr = user_obj as usize;
    let end_vaddr = start_vaddr + size_of::<T>() - 1;
    TASK.get().unwrap().manually_alloc_range(start_vaddr, end_vaddr)
}

/// 输出微秒形式的时间统计，用于调试
pub fn raw_time() -> (usize, usize) {
    TASK.get().unwrap().raw_time()
}

/// 以 TimeVal 字段格式输出计时器信息，第一个是 timer_interval_us，第二个是 timer_remained_us
pub fn raw_timer() -> (usize, usize) {
    TASK.get().unwrap().raw_timer()
}

/// 以 TimeVal 字段格式形式读入计时器信息，返回是否设置成功(类型参数对就算设置成功)
pub fn set_timer(timer_interval_us: usize, timer_remained_us: usize, timer_type: usize) -> bool {
    TASK.get().unwrap().set_timer(timer_interval_us, timer_remained_us, timer_type)
}