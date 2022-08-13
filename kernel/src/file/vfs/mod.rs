//! 虚拟文件系统管理
//! 用户堆一些特殊目录和文件的访问，如 /dev/zero 或 /tmp

mod null;
mod temp;
mod virt_dir;
mod virt_file;
mod zero;

use alloc::{string::String, sync::Arc};
use lock::Mutex;
// 其实这里不要求有序性，可以不用 BTree。
// 但 std::collections::HashMap 不是那么容易在 no_std 下找到，需要引入依赖库
// 所以方便起见就不用 HashMap 了
use super::{File, Kstat, OpenFlags};
use alloc::collections::BTreeMap;
use null::NullFile;
use virt_dir::VirtDir;
use virt_file::{VirtFile, VirtFileInner};
pub type BufferFile = VirtFileInner;
use zero::ZeroFile;

lazy_static::lazy_static! {
    /// 属于虚拟文件系统的目录
    static ref VFS_DIRS: Mutex<BTreeMap<String, Arc<VirtDir>>> = Mutex::new({
        let mut dirs:BTreeMap<String, Arc<VirtDir>> = BTreeMap::new();
        dirs.insert(String::from("dev"), Arc::new({
            let dev = VirtDir::new(String::from("dev"));
            dev.create_file(&String::from("null"), Arc::new(NullFile));
            dev.create_file(&String::from("zero"), Arc::new(ZeroFile));
            dev
        }));
        dirs.insert(String::from("tmp"), Arc::new({
            VirtDir::new(String::from("tmp"))
        }));
        dirs.insert(String::from("var/tmp"), Arc::new({
            VirtDir::new(String::from("var/tmp"))
        }));
        dirs
    });
}

/// 查询这个目录是否是 vfs 里的目录，如果是则从 vfs 中取对应文件
pub fn get_virt_file_if_possible(dir: &String, file: &String, flags: OpenFlags) -> Option<Arc<dyn File>> {
    match VFS_DIRS
        .lock()
        .get(dir.strip_prefix("./")?.strip_suffix("/")?)
    {
        // 找到了说明是 vfs 里的目录
        // 这里试图从目录里获取文件。如果 file == ""，则视为需要打开该目录
        Some(virt_dir) => virt_dir.get_file(file, flags),
        None => None,
    }
}

/// 查询这个目录是否是 vfs 里的目录，是则返回对应目录
pub fn get_virt_dir_if_possible(dir: &String) -> Option<Arc<VirtDir>> {
    //println!("check {dir} is vdir");
    match VFS_DIRS
        .lock()
        .get(dir.strip_prefix("./")?.strip_suffix("/")?)
    {
        // 找到了说明是 vfs 里的目录
        Some(virt_dir) => Some(virt_dir.clone()),
        None => None,
    }
}

/// 尝试新建目录，如果成功则创建这个目录，并存入 VFS_DIRS 中
pub fn try_make_virt_dir(dir: &VirtDir, new_dir_name: &String) -> bool {
    if let Some(new_dir) = dir.mkdir(new_dir_name) {
        let name = new_dir.get_name();
        VFS_DIRS.lock().insert(name, new_dir);
        true
    } else {
        false
    }
}

/// 检查是否存在对应文件。Some表示路径存在，true/false表示文件是否存在
pub fn check_virt_file_exists(dir: &String, file_name: &String) -> Option<bool> { // 这里套了 option 是为了方便用问号
    Some(VFS_DIRS.lock().get(dir.strip_prefix("./")?.strip_suffix("/")?)?.check_file_exists(file_name))
}

/// 删除对应文件或目录。Some表示路径存在，true/false表示文件是否存在
pub fn try_remove_virt_file(dir: &String, file_name: &String) -> Option<bool> {
    let mut vfs_dirs = VFS_DIRS.lock();
    let virt_dir = vfs_dirs.get(dir.strip_prefix("./")?.strip_suffix("/")?)?;
    if let Some(file) = virt_dir.remove_file(file_name) {
        if let Some(dir_name) = file.get_dir() { // 说明是个 VirtDir 目录，需要从 VFS_DIRS 里删掉
            vfs_dirs.remove(dir_name);
        }
        Some(true)
    } else {
        Some(false)
    }
}

/// 检查是否存在对应目录
pub fn check_virt_dir_exists(dir: &String) -> Option<bool> { // 这里套了 option 是为了方便用问号
    Some(VFS_DIRS.lock().get(dir.strip_prefix("./")?.strip_suffix("/")?).is_some())
}
