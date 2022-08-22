//! 整个文件系统的信息

use crate::file::FsStat;

/// 把信息写入 stat 中
pub fn get_fs_stat(stat: *mut FsStat) {
    unsafe {
        (*stat).f_type = 0;
        (*stat).f_bsize = 512; // 其实可以从 fs 实际的 crate 中获取这些常量
        (*stat).f_blocks = 0x4000_0000 / 512; // 但是跟 kernel 不在一个 crate 里
        (*stat).f_bfree = 1; // 太麻烦了就直接写 magic number 了
        (*stat).f_bavail = 1;
        (*stat).f_files = 1;
        (*stat).f_ffree = 1;
        (*stat).f_fsid = [0, 0];
        (*stat).f_namelen = 256; // 命名长度限制默认是 256
        (*stat).f_frsize = 0x1000;
        (*stat).f_flags = 0;
        (*stat).f_spare = [0, 0, 0, 0];
    }
}
