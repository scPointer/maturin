//! 文件类抽象，包含文件系统、stdin/stdout、管道等

#![deny(missing_docs)]

use alloc::vec::Vec;
use alloc::sync::Arc;
pub use fatfs::SeekFrom;

mod fd_manager;
mod os_inode;
mod device;
mod stdio;
mod pipe;
mod kstat;

/// 文件类抽象
pub trait File: Send + Sync {
    /// 读文件内容到 buf，返回读到的字节数。
    /// 如文件不可读，返回 None。(相对应地，如果可读但没有读到内容，返回 Some(0))
    fn read(&self, buf: &mut [u8]) -> Option<usize>;
    /// 写 buf 中的内容到文件中，返回写入的字节数。
    /// 如文件不可写，返回 None。(相对应地，如果可写但无法继续写入内容，返回 Some(0))
    fn write(&self, buf: &[u8]) -> Option<usize>;
    /// 切换当前指针，返回切换后指针到文件开头的距离
    /// 如果文件本身不支持 seek(如pipe，是FIFO"设备") 则返回 None
    fn seek(&self, seekfrom: SeekFrom) -> Option<usize> {
        None
    }
    /// 获取路径。
    /// - 专为 FsDir 设计。因为 linux 的 sys_openat 需要目录的文件描述符，但目录本身不能直接读写，所以特地开一个函数
    /// - 其他 File 类型返回 None 即可
    fn get_dir(&self) -> Option<&str> {
        None
    }
    /// 读取全部数据。
    /// 不是所有类型都实现了 read_all，目前只有文件系统中的文件是可知明确"大小"的，所以可以读"all"。
    /// 对于其他类型来说，这个函数没有实现。
    /// 调用者需要保证这个文件确实可以明确知道"大小"，所以它是 unsafe 的
    unsafe fn read_all(&self) -> Vec<u8> {
        unimplemented!();
    }
    /// 获取文件状态并写入 stat。成功时返回 true。
    /// 
    /// 目前只有fat文件系统中的文件会处理这个函数
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        false
    }
}

pub use stdio::{Stdin, Stdout, Stderr};
pub use fd_manager::FdManager;
pub use pipe::Pipe;
/*
pub use os_inode::{OSInode, OpenFlags};
pub use os_inode::{
    list_apps_names_at_root_dir,
    open_file,
    check_file_exists, 
};
*/
pub use kstat::{Kstat, StMode};
pub use kstat::normal_file_mode;
pub use device::{OpenFlags};
pub use device::{
    list_files_at_root,
    open_file,
    check_file_exists,
    check_dir_exists,
    //load_testcases,
    load_next_testcase,
    umount_fat_fs,
    mount_fat_fs,
    mkdir,
    try_remove_link,
    try_add_link,
    get_kth_dir_entry_info_of_path,
};
