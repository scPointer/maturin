#![no_std]

extern crate alloc;

mod flags;
pub use flags::{EpollEvent, EpollEventType, EpollCtl};
mod epoll_file;
pub use epoll_file::EpollFile;


/// 错误编号
#[repr(C)]
#[derive(Debug)]
pub enum EpollErrorNo {
    /// 找不到文件或目录
    ENOENT = -2,
    /// 文件已存在
    EEXIST = -17,
}