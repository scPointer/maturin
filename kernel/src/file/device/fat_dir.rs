//! FAT中的目录抽象。
//! 主要是包装实现 Trait File

#![deny(missing_docs)]

use lock::Mutex;
use alloc::sync::Arc;
use alloc::string::String;

use super::File;
use super::FsDir;

/// 把 FsDir 包装一层以适应 Trait File
pub struct FatDir {
    /// 是否可读
    pub readable: bool,
    /// 是否可写
    pub writable: bool,
    /// 目录的路径，相对于根目录
    pub dir: String,
    /// 内部结构
    inner: Arc<Mutex<FsDir>>,
}

