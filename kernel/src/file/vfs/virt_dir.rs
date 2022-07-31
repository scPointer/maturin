//! 虚拟文件系统的目录。不需要考虑把数据塞进页里
//! 

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lock::Mutex;
use super::File;

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
}

impl VirtDir {
    /// 创建目录
    pub fn new() -> Self {
        Self {
            entry: Vec::new(),
        }
    }
    /// 检查文件是否存在，如存在则返回一个 Arc 引用
    pub fn get_file(&self, name: &String) -> Option<Arc<dyn File>> {
        self.entry.iter().find(|&e| e.name == *name).map(|e| e.file.clone())
    }
    /// 创建文件，返回是否成功
    pub fn create_file(&mut self, name: &String, file: Arc<dyn File>) -> bool {
        if self.get_file(name).is_some() { // 文件已存在
            false
        } else {
            self.entry.push(DirEntry::new(name.clone(), file));
            true
        }
    }
}