//! 虚拟文件系统的目录。不需要考虑把数据塞进页里
//!

use crate::file::{normal_file_mode, File, Kstat, StMode};
use alloc::{string::String, sync::Arc, vec::Vec};

/// 目录项
pub struct DirEntry {
    pub name: String,
    pub file: Arc<dyn File>,
}

impl DirEntry {
    // 创建新的目录项
    pub fn new(name: String, file: Arc<dyn File>) -> Self {
        Self {
            name: name,
            file: file,
        }
    }
}

/// 虚拟目录。目前暂时不支持更快的查找，仍像普通fs那样按顺序查找
pub struct VirtDir {
    entry: Vec<DirEntry>,
    name: String,
}

impl VirtDir {
    /// 创建目录
    pub fn new(name: String) -> Self {
        Self {
            entry: Vec::new(),
            name: name,
        }
    }
    /// 检查文件是否存在，如存在则返回一个 Arc 引用
    pub fn get_file(&self, name: &String) -> Option<Arc<dyn File>> {
        self.entry
            .iter()
            .find(|&e| e.name == *name)
            .map(|e| e.file.clone())
    }
    /// 创建文件，返回是否成功
    pub fn create_file(&mut self, name: &String, file: Arc<dyn File>) -> bool {
        if self.get_file(name).is_some() {
            // 文件已存在
            false
        } else {
            self.entry.push(DirEntry::new(name.clone(), file));
            true
        }
    }
}

impl File for VirtDir {
    /// 路径本身不可读
    fn read(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
    /// 路径本身不可写
    fn write(&self, _buf: &[u8]) -> Option<usize> {
        None
    }
    /// 获取路径
    fn get_dir(&self) -> Option<&str> {
        Some(self.name.as_str())
    }
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        unsafe {
            (*stat).st_dev = 2;
            (*stat).st_ino = 0;
            (*stat).st_mode = normal_file_mode(StMode::S_IFDIR).bits();
            (*stat).st_nlink = 1;
            (*stat).st_size = 0;
            (*stat).st_uid = 0;
            (*stat).st_gid = 0;
            (*stat).st_atime_sec = 0;
            (*stat).st_atime_nsec = 0;
            (*stat).st_mtime_sec = 0;
            (*stat).st_mtime_nsec = 0;
            (*stat).st_ctime_sec = 0;
            (*stat).st_ctime_nsec = 0;
        }
        true
    }
}
