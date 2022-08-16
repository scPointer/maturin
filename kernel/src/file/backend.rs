//! 和某段内存同步的实际后端文件，带有一个偏移量，相当于原文件的某一段
//! 可以根据需要和源文件同步

use alloc::sync::Arc;
use super::File;

/// 同步策略(本来想搞类型体操，但太花了
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum SyncPolicy {
    // 同步读
    SyncRead,
    // 同步写
    SyncWrite,
    // 读写均同步
    SyncReadWrite,
}

/// 和 mmap 段同步，只能通过 read_from_offset 和 write_to_offset 读写
pub struct BackEndFile {
    file: Arc<dyn File>,
    offset: usize,
    policy: SyncPolicy,
}

impl BackEndFile {
    /// 创建时不检查 offset 是否合法
    pub fn new(file: Arc<dyn File>, offset: usize, policy: SyncPolicy) -> Self {
        Self {
            file: file,
            offset: offset,
            policy: policy,
        }
    }
    /// 当区间因为 mmap / munmap 被切分时，映射的后端文件也要同步切分。
    /// 返回一个映射到同文件，但是偏移为 self.offset + delta 的后端文件
    pub fn split(&self, delta: usize) -> Self {
        Self {
            file: self.file.clone(),
            offset: self.offset + delta,
            policy: self.policy,
        }
    }
    /// 复制一份后端文件，一般是 fork 要求的
    pub fn clone_as_fork(&self) -> Self {
        self.split(0)
    }
    /// 改变这个后端文件所映射的文件的偏移量。通常是由于 mmap / munmap / mprotect 导致的区间改变
    pub fn modify_offset(&mut self, delta: usize) {
        self.offset += delta;
    }
}

impl File for BackEndFile {
    /// 后端不可直接读
    fn read(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
    /// 不可直接写
    fn write(&self, _buf: &[u8]) -> Option<usize> {
        None
    }
    /// 转移读操作
    fn read_from_offset(&self, pos: usize, buf: &mut [u8]) -> Option<usize> {
        if self.policy == SyncPolicy::SyncRead || self.policy == SyncPolicy::SyncReadWrite {
            //println!("backend read self.offset {:x} pos {:x}", self.offset, pos);
            self.file.read_from_offset(self.offset + pos, buf)
        } else {
            None
        }
    }
    /// 转移写操作
    fn write_to_offset(&self, pos: usize, buf: &[u8]) -> Option<usize> {
        if self.policy == SyncPolicy::SyncWrite || self.policy == SyncPolicy::SyncReadWrite {
            //println!("backend write self.offset {:x} pos {:x}", self.offset, pos);
            self.file.write_to_offset(self.offset + pos, buf)
        } else {
            None
        }
    }
}
