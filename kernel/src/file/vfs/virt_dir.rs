//! 虚拟文件系统的目录。不需要考虑把数据塞进页里
//!

use alloc::{string::String, sync::Arc, vec::Vec};
use lock::Mutex;
use crate::file::{File, Kstat, StMode, OpenFlags, normal_file_mode};
use super::VirtFile;

/// 目录项
pub struct DirEntry {
    pub name: String,
    pub file: Arc<dyn File>,
}

impl DirEntry {
    // 创建新的目录项
    pub fn new(name: String, file: Arc<dyn File>) -> Self {
        Self {
            name: name,
            file: file,
        }
    }
}

/// 虚拟目录。目前暂时不支持更快的查找，仍像普通fs那样按顺序查找
pub struct VirtDir {
    entry: Mutex<Vec<DirEntry>>,
    name: String,
}

impl VirtDir {
    /// 创建目录
    pub fn new(name: String) -> Self {
        Self {
            entry: Mutex::new(Vec::new()),
            name: name,
        }
    }
    /// 获取目录名字
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
    /// 检查文件是否存在
    pub fn check_file_exists(&self, file_name: &String) -> bool {
        self.entry.lock().iter().find(|&e| e.name == *file_name).is_some()
    }
    /// 删除一个文件
    pub fn remove_file(&self, file_name: &String) -> Option<Arc<dyn File>> {
        let entry:Vec<DirEntry> = self.entry.lock().drain_filter(|e| e.name == *file_name).collect();
        if entry.len() == 0 { //没找到文件
            None
        } else {
            //entry[0].file.clear();
            Some(entry[0].file.clone())
        }
    }
    /// 检查文件是否存在，如存在则返回一个 Arc 引用
    pub fn get_file(self: &Arc<Self>, file_name: &String, flags: OpenFlags) -> Option<Arc<dyn File>> {
        //println!("vdir get file {file_name} {:#?}", flags);
        if flags.contains(OpenFlags::DIR) || flags.contains(OpenFlags::DSYNC) || file_name.len() == 0 {
            if file_name.len() == 0 { // 要求返回自己
                Some(self.clone())
            } else {
                self.entry.lock().iter().find(|&e| e.name == *file_name).map(|e| e.file.clone())
            }
        } else {
            let mut self_entry = self.entry.lock();
            match self_entry.iter().find(|&e| e.name == *file_name).map(|e| e.file.clone()) {
                Some(f) => {
                    if flags.contains(OpenFlags::EXCL) {
                        //要求必须要创建文件
                        None
                    } else {
                        if flags.contains(OpenFlags::CREATE) {
                            // 清空这个文件
                            f.clear();
                        };
                        Some(f)
                    }
                },
                None => {
                    // 找不到且要求创建，则默认创建 VirtFile
                    if flags.contains(OpenFlags::CREATE) {
                        let file:Arc<dyn File> = Arc::new(VirtFile::new(flags));
                        let ret = file.clone();
                        self_entry.push(DirEntry::new(file_name.clone(), file));
                        Some(ret)
                    } else {
                        None
                    }
                },
            }
        }
    }
    /// 创建目录。如果创建成功，则返回新建的目录
    /// 
    /// 使用上层的 **/vfs/mod.rs: try_mkdir()** ，而不要直接调用这个函数，因为新目录需要放进 VIRT_DIRS 里
    pub fn mkdir(&self, dir_name: &String) -> Option<Arc<VirtDir>> {
        //println!("vdir mkdir {dir_name}");
        if self.check_file_exists(dir_name) {
            None
        } else {
            let dir:Arc<VirtDir> = Arc::new(VirtDir::new(self.name.clone() + "/" + dir_name.as_str()));
            let ret = dir.clone();
            self.entry.lock().push(DirEntry::new(dir_name.clone(), dir));
            Some(ret)
        }
    }
    /// 创建文件，返回是否成功
    pub fn create_file(&self, name: &String, file: Arc<dyn File>) -> bool {
        if self.check_file_exists(name) {
            // 文件已存在
            false
        } else {
            self.entry.lock().push(DirEntry::new(name.clone(), file));
            true
        }
    }
}

impl File for VirtDir {
    /// 路径本身不可读
    fn read(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
    /// 路径本身不可写
    fn write(&self, _buf: &[u8]) -> Option<usize> {
        None
    }
    /// 获取路径
    fn get_dir(&self) -> Option<&str> {
        Some(self.name.as_str())
    }
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        unsafe {
            (*stat).st_dev = 2;
            (*stat).st_ino = 0;
            (*stat).st_mode = normal_file_mode(StMode::S_IFDIR).bits();
            (*stat).st_nlink = 1;
            (*stat).st_size = 0;
            (*stat).st_uid = 0;
            (*stat).st_gid = 0;
            (*stat).st_atime_sec = 0;
            (*stat).st_atime_nsec = 0;
            (*stat).st_mtime_sec = 0;
            (*stat).st_mtime_nsec = 0;
            (*stat).st_ctime_sec = 0;
            (*stat).st_ctime_nsec = 0;
        }
        true
    }
}
