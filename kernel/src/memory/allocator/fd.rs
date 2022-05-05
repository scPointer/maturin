//! 文件描述符分配器
//! 
//! 它与其他分配器的最大特点在于并不是全局只有一个，而是每个进程的 FdManager 都持有一个。也因此：
//! 1. FdAllocator 自身不带 Mutex 锁
//! 2. FdAllocator 的分配和释放直接传递 usize，不包装生命周期，其正确性由外层 FdManager 保证
//! 
//! 目前固定支持最多 256 个 fd，不过也可之后修改

#![deny(missing_docs)]

extern crate bitmap_allocator;
type PidAllocatorImpl = bitmap_allocator::BitAlloc256;
use bitmap_allocator::BitAlloc;

use crate::constants::FD_LIMIT;

pub struct FdAllocator(PidAllocatorImpl);

impl FdAllocator {
    pub fn new() -> Self {
        let mut fd_allocator = Self(PidAllocatorImpl::DEFAULT);
        fd_allocator.0.insert(0..FD_LIMIT);
        fd_allocator
    }
    #[allow(unused)]
    pub fn alloc(&mut self) -> Option<usize> {
        self.0.alloc()
    }
    #[allow(unused)]
    pub fn dealloc(&mut self, fd: usize) {
        self.0.dealloc(fd)
    }
}