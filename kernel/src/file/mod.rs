//! 文件类抽象，包含文件系统、stdin/stdout、管道等

#![deny(missing_docs)]

mod fd_manager;
mod os_inode;
mod device;
mod stdio;
mod pipe;

/// 文件类抽象
pub trait File: Send + Sync {
    /// 读文件内容到 buf，返回读到的字节数。
    /// 如文件不可读，返回 None。(相对应地，如果可读但没有读到内容，返回 Some(0))
    fn read(&self, buf: &mut [u8]) -> Option<usize>;
    /// 写 buf 中的内容到文件中，返回写入的字节数。
    /// 如文件不可写，返回 None。(相对应地，如果可写但无法继续写入内容，返回 Some(0))
    fn write(&self, buf: &[u8]) -> Option<usize>;
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
pub use device::{OpenFlags};
pub use device::{
    list_files_at_root,
    open_file,
    check_file_exists,
    load_testcases,
};
