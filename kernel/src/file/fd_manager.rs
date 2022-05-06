//! 文件描述符管理
//! 内部保存你所有 FD 的 Arc 指针以及一个 FdAllocator

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::memory::FdAllocator;
use crate::error::{OSResult, OSError};

use super::File;
use super::{Stdin, Stdout, Stderr};

/// 文件描述符管理，每个进程应该有一个
/// 这个结构 Drop 时会
pub struct FdManager {
    files: Vec<Option<Arc<dyn File>>>,
    fd_allocator: FdAllocator,
}

impl FdManager {
    /// 新建 FdManager 并插入 Stdin / Stdout / Stderr
    pub fn new() -> Self {
        let mut fd_manager = Self {
            files: Vec::new(),
            fd_allocator: FdAllocator::new(),
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
            fd_allocator: FdAllocator::new(),
        };
        new_manager.files.resize(self.files.len(), None);
        for fd in 0..self.files.len() {
            match &self.files[fd] {
                // 为了构造相同的 fd_allocator，这里需要先 alloc，等复制完 files 再 dealloc
                None => { new_manager.fd_allocator.alloc(); }
                Some(file) => { new_manager.push(file.clone()); }
            }
        }
        for fd in 0..self.files.len() {
            if self.files[fd].is_none() {
                // 为了构造相同的 fd_allocator，这里需要手动 dealloc
                new_manager.fd_allocator.dealloc(fd);
            }
        }
        new_manager
    }
    /// 复制一个 fd 中的文件到新的 fd 上
    pub fn copy_fd(&mut self, old_fd: usize) -> OSResult<usize> {
        self.push(self.get_file(old_fd)?)
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
}
