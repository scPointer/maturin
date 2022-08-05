//! 文件描述符分配器
//!
//! 它与其他分配器的最大特点在于并不是全局只有一个，而是每个进程的 FdManager 都持有一个。也因此：
//! 1. FdAllocator 自身不带 Mutex 锁
//! 2. FdAllocator 的分配和释放直接传递 usize，不包装生命周期，其正确性由外层 FdManager 保证
//!
//! 目前固定支持最多 256 个 fd，不过也可之后修改

//#![deny(missing_docs)]

extern crate bitmap_allocator;
type PidAllocatorImpl = bitmap_allocator::BitAlloc256;
use bitmap_allocator::BitAlloc;

pub struct FdAllocator(PidAllocatorImpl);

impl FdAllocator {
    pub fn new(limit: usize) -> Self {
        let mut fd_allocator = Self(PidAllocatorImpl::DEFAULT);
        fd_allocator.0.insert(0..limit);
        fd_allocator
    }
    pub fn expand_range(&mut self, left: usize, right: usize) {
        self.0.insert(left..right);
    }
    pub fn shrink_range(&mut self, left: usize, right: usize) {
        self.0.remove(left..right);
    }
    #[allow(unused)]
    pub fn alloc(&mut self) -> Option<usize> {
        self.0.alloc()
    }
    #[allow(unused)]
    pub fn dealloc(&mut self, fd: usize) {
        self.0.dealloc(fd)
    }
    #[allow(unused)]
    pub fn is_allocated(&mut self, fd: usize) -> bool {
        !self.0.test(fd)
    }
    #[allow(unused)]
    /// 分配一个确定的 fd。这个函数不检查 fd 是否已经被分配
    pub unsafe fn alloc_exact(&mut self, fd: usize) {
        self.0.remove(fd..fd + 1)
    }
    #[allow(unused)]
    /// 尝试分配 fd，如果成功(即该 fd 之前没有被分配)，则返回 true
    pub fn alloc_exact_if_possible(&mut self, fd: usize) -> bool {
        if !self.is_allocated(fd) {
            unsafe {
                self.alloc_exact(fd);
            }
            true
        } else {
            false
        }
    }
}
