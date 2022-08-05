//! 虚拟文件系统管理
//! 用户堆一些特殊目录和文件的访问，如 /dev/zero 或 /tmp

mod null;
mod virt_dir;
mod zero;

use alloc::{string::String, sync::Arc};
use lazy_static::*;
use lock::Mutex;
// 其实这里不要求有序性，可以不用 BTree。
// 但 std::collections::HashMap 不是那么容易在 no_std 下找到，需要引入依赖库
// 所以方便起见就不用 HashMap 了
use super::{File, Kstat};
use alloc::collections::BTreeMap;
use null::NullFile;
use virt_dir::VirtDir;
use zero::ZeroFile;

lazy_static! {
    /// 属于虚拟文件系统的目录
    static ref VFS_DIRS: Mutex<BTreeMap<String, VirtDir>> = Mutex::new({
        let mut dirs:BTreeMap<String, VirtDir> = BTreeMap::new();
        dirs.insert(String::from("dev"), {
            let mut dev = VirtDir::new();
            dev.create_file(&String::from("null"), Arc::new(NullFile));
            dev.create_file(&String::from("zero"), Arc::new(ZeroFile));
            dev
        });
        dirs
    });
}

/// 查询这个目录是否是 vfs 里的目录，如果是则从 vfs 中取对应文件
pub fn get_virt_file_if_possible(dir: &String, file: &String) -> Option<Arc<dyn File>> {
    match VFS_DIRS
        .lock()
        .get(dir.strip_prefix("./")?.strip_suffix("/")?)
    {
        // 找到了说明是 vfs 里的目录
        Some(virt_dir) => virt_dir.get_file(file),
        None => None,
    }
}
