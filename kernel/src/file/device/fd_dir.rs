//! 以文件描述符形式保存一个路径。
//! 主要用于 sys_openat ，这个系统调用需要文件所在目录的 fd

#![deny(missing_docs)]

use alloc::string::String;

use super::File;

/// 仅保存路径的文件描述符实现
pub struct FdDir {
    /// 路径本体，占用堆空间保存
    dir: String,
}

impl FdDir {
    /// 初始化一个 FdDir，如果输入的目录没有 '/' 结尾，则自动添加
    pub fn new(dir: String) -> Self {
        let mut dir = dir;
        if !dir.ends_with("/") {
            dir.push('/');
        }
        Self {
            dir: dir
        }
    }
}

impl File for FdDir {
    /// 路径本身不可读
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        None
    }
    /// 路径本身不可写
    fn write(&self, buf: &[u8]) -> Option<usize> {
        None
    }
    /// 获取路径
    fn get_dir(&self) -> Option<&str> {
        Some(self.dir.as_str())
    }
}