//! 以文件描述符形式保存一个路径。
//! 主要用于 sys_openat ，这个系统调用需要文件所在目录的 fd

//#![deny(missing_docs)]

use alloc::string::String;

use super::File;
use crate::file::{normal_file_mode, Kstat, StMode};

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
        Self { dir: dir }
    }
}

impl File for FdDir {
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
        Some(self.dir.as_str())
    }
    /// 可修改 CLOEXEC 信息
    fn set_close_on_exec(&self, _is_set: bool) -> bool {
        true
    }
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        unsafe {
            (*stat).st_dev = 1;
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
