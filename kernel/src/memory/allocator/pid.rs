//! PID 分配器
//! 最大支持 4096 个进程id。如需要更多，修改下面的 PidAllocatorImpl 即可
//! 实际上u740板子的内存16G用不完bit map，但再小一点的实现只有4G空间

#![deny(missing_docs)]


extern crate bitmap_allocator;
type PidAllocatorImpl = bitmap_allocator::BitAlloc4K;
use bitmap_allocator::BitAlloc;
use lock::Mutex;

use crate::constants::{PID_LIMIT};


static PID_ALLOCATOR: Mutex<PidAllocatorImpl> = Mutex::new(PidAllocatorImpl::DEFAULT);

/// 从 PID 分配器中分配一个 usize
unsafe fn alloc_pid_raw() -> Option<usize> {
    PID_ALLOCATOR.lock().alloc()
}

/// 释放一个 usize，它必须已经被分配过
unsafe fn dealloc_pid_raw(pid: usize) {
    PID_ALLOCATOR.lock().dealloc(pid)
}

/// 分配一个 usize，并打包成 Pid
pub fn alloc_pid() -> Option<Pid> {
    Some(Pid(PID_ALLOCATOR.lock().alloc()?))
}

/// 初始化 pid 分配
pub fn init() {
    PID_ALLOCATOR.lock().insert(1..PID_LIMIT)
}

/// 保存一个 PID ，当 Drop 时会自动释放
#[derive(Debug)]
pub struct Pid(pub usize);

impl Pid {
    /// 分配一个 pid，如没有可用的 pid，则返回 None
    pub fn new() -> Option<Self> {
        alloc_pid()
    }
}

impl Drop for Pid {
    /// 释放这个 pid，它可用于其他进程
    fn drop(&mut self) {
        unsafe { dealloc_pid_raw(self.0) }
    }
}
