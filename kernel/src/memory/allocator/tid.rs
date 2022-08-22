//! TID 分配器
//! 最大支持 4096 个线程id。如需要更多，修改下面的 TidAllocatorImpl 即可
//! 实际上u740板子的内存16G用不完bit map，但再小一点的实现只有4G空间

//#![deny(missing_docs)]

extern crate bitmap_allocator;
type TidAllocatorImpl = bitmap_allocator::BitAlloc4K;
use bitmap_allocator::BitAlloc;
use lock::Mutex;

use crate::constants::TID_LIMIT;

static TID_ALLOCATOR: Mutex<TidAllocatorImpl> = Mutex::new(TidAllocatorImpl::DEFAULT);

/// 从 TID 分配器中分配一个 usize
#[allow(dead_code)]
unsafe fn alloc_tid_raw() -> Option<usize> {
    TID_ALLOCATOR.lock().alloc()
}

/// 释放一个 usize，它必须已经被分配过
unsafe fn dealloc_tid_raw(tid: usize) {
    TID_ALLOCATOR.lock().dealloc(tid)
}

/// 分配一个 usize，并打包成 Tid
pub fn alloc_tid() -> Option<Tid> {
    Some(Tid(TID_ALLOCATOR.lock().alloc()?))
}

/// 初始化 tid 分配
pub fn init() {
    TID_ALLOCATOR.lock().insert(2..TID_LIMIT)
}

/// 保存一个 TID ，当 Drop 时会自动释放
#[derive(Debug)]
pub struct Tid(pub usize);

impl Tid {
    /// 分配一个 tid，如没有可用的 tid，则返回 None
    pub fn new() -> Option<Self> {
        alloc_tid()
    }
}

impl Drop for Tid {
    /// 释放这个 tid，它可用于其他进程
    fn drop(&mut self) {
        unsafe { dealloc_tid_raw(self.0) }
    }
}
