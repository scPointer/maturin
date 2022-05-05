//! 文件系统 inode 的抽象

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::*;
use lock::Mutex;

use crate::drivers::BLOCK_DEVICE;

use super::File;
/// 把 inode 包装一层以适应 Trait File
pub struct OSInode {
    /// (对构造这个 inode 的进程来说) 是否可读
    pub readable: bool,
    /// (对构造这个 inode 的进程来说) 是否可写
    pub writable: bool,
    /// 内部结构
    inner: Mutex<OSInodeInner>,
}
/// 内部可变部分
pub struct OSInodeInner {
    /// 在文件中的偏移
    offset: usize,
    /// easy-fs 库定义的 Inode
    inode: Arc<Inode>,
}

impl OSInode {
    /// 从 easy-fs 库中的 inode 构造
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: Mutex::new(OSInodeInner { offset: 0, inode }),
        }
    }
    /// 读取所有数据
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

lazy_static! {
    /// 根目录所在的 inode
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

/// 列出根目录下所有文件名
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {    
    /// 指定文件打开时的权限
    pub struct OpenFlags: u32 {
        /// 只读
        const RDONLY = 0;
        /// 只能写入
        const WRONLY = 1 << 0;
        /// 读写
        const RDWR = 1 << 1;
        /// 如文件不存在，可创建它
        const CREATE = 1 << 9;
        /// 清空文件，然后新建一个同名的
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// 获得文件的读/写权限
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}
/// 打开文件
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create file
            ROOT_INODE
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

impl File for OSInode {
    /// 读取文件，并移动 offset
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        let mut inner = self.inner.lock();
        let read_size = inner.inode.read_at(inner.offset, buf);
        inner.offset += read_size;
        Some(read_size)
    }
    /// 写入文件，并移动 offset
    fn write(&self, buf: &[u8]) -> Option<usize> {
        let mut inner = self.inner.lock();
        let write_size = inner.inode.write_at(inner.offset, buf);
        // 这里假设文件一定能按要求写完所有内容
        assert_eq!(write_size, buf.len());
        inner.offset += write_size;
        Some(write_size)
    }
}
