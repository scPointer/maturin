//! 文件描述符管理
//! 内部保存你所有 FD 的 Arc 指针以及一个 FdAllocator

//#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::constants::{FD_LIMIT_ORIGIN, FD_LIMIT_HARD};
use crate::error::{OSError, OSResult};
use crate::memory::FdAllocator;

use super::{File, OpenFlags};
use super::{Stderr, Stdin, Stdout};

/// 文件描述符管理，每个进程应该有一个
/// 这个结构 Drop 时会自动释放文件的 Arc
pub struct FdManager {
    /// 内部包含的文件
    files: Vec<Option<Arc<dyn File>>>,
    /// 描述符和分配器
    fd_allocator: FdAllocator,
    /// 最大 fd 限制
    limit: usize,
}

impl FdManager {
    /// 新建 FdManager 并插入 Stdin / Stdout / Stderr
    pub fn new() -> Self {
        let limit = FD_LIMIT_ORIGIN;
        let mut fd_manager = Self {
            files: Vec::new(),
            fd_allocator: FdAllocator::new(limit),
            limit: limit,
        };
        fd_manager.push(Arc::new(Stdin)).unwrap();
        fd_manager.push(Arc::new(Stdout)).unwrap();
        fd_manager.push(Arc::new(Stderr)).unwrap();
        fd_manager
    }
    /// 从另一个 FdManager 复制一份文件描述符表。
    ///
    /// Todo:
    /// 目前因为 FdAllocator 依赖的 bitmap_allocator 是不可复制的，
    /// 所以此处想要手动获得一个跟原来一样的 fd_allocator 比较麻烦。
    /// 最好重新实现一下 FdAllocator
    pub fn copy_all(&self) -> Self {
        let mut new_manager = Self {
            files: Vec::new(),
            fd_allocator: FdAllocator::new(self.limit),
            limit: self.limit,
        };
        new_manager.files.resize(self.files.len(), None);
        for fd in 0..self.files.len() {
            // 现在可以直接分配特定 fd 了
            if let Some(file) = &self.files[fd] {
                // 暴力分配 fd。
                // 因为我们知道新创建的 new_manager 是空的，但 fd_allocator 自己不知道，所以要 unsafe
                unsafe {
                    new_manager.fd_allocator.alloc_exact(fd);
                }
                new_manager.files[fd] = Some(file.clone());
            }
            /*
            match &self.files[fd] {
                // 为了构造相同的 fd_allocator，这里需要先 alloc，等复制完 files 再 dealloc
                None => { new_manager.fd_allocator.alloc(); }
                Some(file) => { new_manager.push(file.clone()); }
            }
            */
        }
        /*
        for fd in 0..self.files.len() {
            if self.files[fd].is_none() {
                // 为了构造相同的 fd_allocator，这里需要手动 dealloc
                new_manager.fd_allocator.dealloc(fd);
            }
        }
        */
        new_manager
    }
    /// 复制一个 fd 中的文件到新的 fd 上
    pub fn copy_fd_anywhere(&mut self, old_fd: usize) -> OSResult<usize> {
        self.push(self.get_file(old_fd)?)
    }
    /// 复制一个 fd 到指定的新 fd 上，返回是否成功
    pub fn copy_fd_to(&mut self, old_fd: usize, new_fd: usize) -> bool {
        self.get_file(old_fd)
            .map(|file| {
                self.fd_allocator.alloc_exact_if_possible(new_fd);
                // 因为已经分配了，所以不走 self.push
                if self.files.len() <= new_fd {
                    self.files.resize(new_fd + 1, None);
                }
                // 这里可能会删除该处原有的fd，不过这是符合语义的
                self.files[new_fd].replace(file);
            })
            .is_ok()
    }
    /// 插入一个新文件
    pub fn push(&mut self, file: Arc<dyn File>) -> OSResult<usize> {
        if let Some(fd) = self.fd_allocator.alloc() {
            // 如果 files 不够长，扩展它直到能放下 files[fd]
            if self.files.len() <= fd {
                self.files.resize(fd + 1, None);
            }
            self.files[fd] = Some(file);
            Ok(fd)
        } else {
            Err(OSError::FdManager_NoAvailableFd)
        }
    }
    /// 拿到一个文件的 Arc 指针(clone 语义)
    pub fn get_file(&self, fd: usize) -> OSResult<Arc<dyn File>> {
        if fd >= self.files.len() || self.files[fd].is_none() {
            return Err(OSError::FdManager_FdNotFound);
        } else {
            Ok(self.files[fd].as_ref().unwrap().clone())
        }
    }
    
    /// 检查是否 vec 里所有 fd 都存在，如果存在则返回它们对应的文件，否则返回 None
    pub fn get_files_if_all_exists(&self, vec: &Vec<usize>) -> Option<Vec<Arc<dyn File>>> {
        let mut files: Vec<Arc<dyn File>> = Vec::with_capacity(vec.len());
        for &fd in vec {
            if let Ok(file) = self.get_file(fd) {
                files.push(file);
            } else {
                return None
            }
        }
        Some(files)
    }
    
    /// 删除一个文件，相当于以 take 语义拿到一个文件的 Arc 指针。
    /// 这个函数还是会检查 fd 是否存在，如不存在，则返回的是 Err
    pub fn remove_file(&mut self, fd: usize) -> OSResult<Arc<dyn File>> {
        if fd >= self.files.len() || self.files[fd].is_none() {
            return Err(OSError::FdManager_FdNotFound);
        } else {
            self.fd_allocator.dealloc(fd);
            Ok(self.files[fd].take().unwrap())
        }
    }
    /// 测试fd是否已经满了
    pub fn is_full(&mut self) -> bool {
        if let Some(fd) = self.fd_allocator.alloc() {
            self.fd_allocator.dealloc(fd);
            false
        } else {
            true
        }
    }
    /// 获取当前 fd 的上限
    pub fn get_limit(&self) -> usize {
        self.limit
    }
    /// 修改当前 fd 的上限
    pub fn modify_limit(&mut self, new_limit: usize) {
        // 上限不能超过最初始的设定，因为分配器的实现是固定的
        let new_limit = new_limit.min(FD_LIMIT_HARD).max(0);
        if new_limit < self.limit {
            self.fd_allocator.shrink_range(new_limit, self.limit);
        } else if new_limit > self.limit {
            self.fd_allocator.expand_range(self.limit, new_limit);
        }
        self.limit = new_limit;
    }
    /// 删除所有带有 CLOEXEC 标记的文件。在 exec 时使用
    pub fn close_cloexec_files(&mut self) {
        // 这里希望删除文件后其他文件顺序不变，所以用枚举 fd 而不是迭代器之类的方法
        for fd in 0..self.files.len() {
            if self.files[fd].is_some() && self.files[fd].as_ref().unwrap().get_status().contains(OpenFlags::CLOEXEC) {
                self.files[fd].take();
            }
        }
    }
}
