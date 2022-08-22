//! FAT中的目录抽象。
//! 主要是包装实现 Trait File

use super::FsDir;
use alloc::{string::String, sync::Arc};
use lock::Mutex;

/// 把 FsDir 包装一层以适应 Trait File
#[allow(dead_code)]
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
