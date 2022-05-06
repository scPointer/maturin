//! 文件类抽象，包含文件系统、stdin/stdout、管道等

#![deny(missing_docs)]

mod fd_manager;
mod os_inode;
mod stdio;

/// 文件类抽象
pub trait File: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> Option<usize>;
    fn write(&self, buf: &[u8]) -> Option<usize>;
}

pub use stdio::{Stdin, Stdout, Stderr};
pub use os_inode::{OSInode, OpenFlags};
pub use os_inode::{
    list_apps_names_at_root_dir,
    open_file,
    check_file_exists, 
};
pub use fd_manager::FdManager;


