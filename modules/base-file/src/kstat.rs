//! 文件信息类
//!
//! 如果文件在文件系统中，应返回具体信息

//#![deny(missing_docs)]

use bitflags::*;

/// 文件信息类
#[repr(C)]
pub struct Kstat {
    /// 设备
    pub st_dev: u64,
    /// inode 编号
    pub st_ino: u64,
    /// 文件类型
    pub st_mode: u32,
    /// 硬链接数
    pub st_nlink: u32,
    /// 用户id
    pub st_uid: u32,
    /// 用户组id
    pub st_gid: u32,
    /// 设备号
    pub st_rdev: u64,
    _pad0: u64,
    /// 文件大小
    pub st_size: u64,
    /// 块大小
    pub st_blksize: u32,
    _pad1: u32,
    /// 块个数
    pub st_blocks: u64,
    /// 最后一次访问时间(秒)
    pub st_atime_sec: isize,
    /// 最后一次访问时间(纳秒)
    pub st_atime_nsec: isize,
    /// 最后一次修改时间(秒)
    pub st_mtime_sec: isize,
    /// 最后一次修改时间(纳秒)
    pub st_mtime_nsec: isize,
    /// 最后一次改变状态时间(秒)
    pub st_ctime_sec: isize,
    /// 最后一次改变状态时间(纳秒)
    pub st_ctime_nsec: isize,
}

bitflags! {
    /// 指定 st_mode 的选项
    pub struct StMode: u32 {
        /// 是普通文件
        const S_IFREG = 1 << 15;
        /// 是目录
        const S_IFDIR = 1 << 14;
        /// 是字符设备
        const S_IFCHR = 1 << 13;
        /// 是否设置 uid/gid/sticky
        //const S_ISUID = 1 << 14;
        //const S_ISGID = 1 << 13;
        //const S_ISVTX = 1 << 12;
        /// 所有者权限
        const S_IXUSR = 1 << 10;
        const S_IWUSR = 1 << 9;
        const S_IRUSR = 1 << 8;
        /// 用户组权限
        const S_IXGRP = 1 << 6;
        const S_IWGRP = 1 << 5;
        const S_IRGRP = 1 << 4;
        /// 其他用户权限
        const S_IXOTH = 1 << 2;
        const S_IWOTH = 1 << 1;
        const S_IROTH = 1 << 0;
        /// 报告已执行结束的用户进程的状态
        const WIMTRACED = 1 << 1;
        /// 报告还未结束的用户进程的状态
        const WCONTINUED = 1 << 3;
    }
}

/// 文件类型，输入 IFCHR / IFDIR / IFREG 等具体类型，
/// 输出这些类型加上普遍的文件属性后得到的 mode 参数
pub fn normal_file_mode(file_type: StMode) -> StMode {
    file_type | StMode::S_IWUSR | StMode::S_IWUSR | StMode::S_IWGRP | StMode::S_IRGRP
}
