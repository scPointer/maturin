//! 文件系统的属性状态，用于 sys_statfs

/// 文件系统的属性
/// 具体参数定义信息来自 `https://man7.org/linux/man-pages/man2/statfs64.2.html`
pub struct FsStat {
    /// 是个 magic number，每个知名的 fs 都各有定义，但显然我们没有
    pub f_type: i64,
    /// 最优传输块大小
    pub f_bsize: i64,
    /// 总的块数
    pub f_blocks: u64,
    /// 还剩多少块未分配
    pub f_bfree: u64,
    /// 对用户来说，还有多少块可用
    pub f_bavail: u64,
    /// 总的 inode 数
    pub f_files: u64,
    /// 空闲的 inode 数
    pub f_ffree: u64,
    /// 文件系统编号，但实际上对于不同的OS差异很大，所以不会特地去用
    pub f_fsid: [i32; 2],
    /// 文件名长度限制，这个OS默认FAT已经使用了加长命名
    pub f_namelen: isize,
    /// 片大小
    pub f_frsize: isize,
    /// 一些选项，但其实也没用到
    pub f_flags: isize,
    /// 空余 padding
    pub f_spare: [isize; 4],
}
