//! 另一种空文件，用于 dev/zero

use super::{File, Kstat};
use crate::file::{normal_file_mode, StMode};

pub struct ZeroFile;

impl File for ZeroFile {
    /// 从 zero 中只会读到0
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        buf.fill(0);
        Some(buf.len())
    }
    /// zero 可写，但没有反馈
    fn write(&self, buf: &[u8]) -> Option<usize> {
        Some(buf.len())
    }
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        unsafe {
            (*stat).st_dev = 0;
            (*stat).st_ino = 0;
            (*stat).st_nlink = 1;
            (*stat).st_mode = normal_file_mode(StMode::S_IFCHR).bits();
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
