//! FAT文件系统设备的抽象
//! 包括读写文件等的支持

#![deny(missing_docs)]

use lazy_static::*;
use lock::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use fatfs::{
    FsOptions,
    FileSystem,
    DefaultTimeProvider, 
    LossyOemCpConverter,
    Read,
    Write,
    Seek,
    SeekFrom,
    Error,
};

use super::File;

use crate::drivers::{new_memory_mapped_fs, MemoryMappedFsIoType};
use crate::constants::ROOT_DIR;

type FsIO = MemoryMappedFsIoType;
type FsTP = DefaultTimeProvider;
type FsOCC = LossyOemCpConverter;

type FsDir = fatfs::Dir<'static, FsIO, FsTP, FsOCC>;
type FsFile = fatfs::File<'static, FsIO, FsTP, FsOCC>;

mod open_flags;
mod fat_file;
mod fat_dir;
mod fd_dir;
mod test;

pub use open_flags::OpenFlags;
pub use fat_file::FatFile;
pub use fat_dir::FatDir;
pub use fd_dir::FdDir;
pub use test::load_testcases;

lazy_static! {
    //static ref MEMORY_FS: Arc<Mutex<FileSystem<FsIO, FsTP, FsOCC>>> = Arc::new(Mutex::new(new_memory_mapped_fs()));
    static ref MEMORY_FS: FileSystem<FsIO, FsTP, FsOCC> = new_memory_mapped_fs();
}

/// 输出根目录下的所有文件
pub fn list_files_at_root() {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    for dir_entry in root.iter() {
        let file = dir_entry.unwrap();
        println!("file: {}", file.file_name());
        // 如果是子目录，则再继续遍历
        if file.is_dir() {
            println!("dir: {}/", file.file_name());
            for dir_entry in root.open_dir(file.file_name().as_str()).unwrap().iter() {
                let file = dir_entry.unwrap();
                // "." 开头的是当前目录、父目录以及(未来可能的)隐藏文件
                if !file.file_name().starts_with(".") {
                    println!("\tfile: {}", file.file_name());
                }
            }
        }
    }
}

/// 分割文件所在路径。返回的第一个值是路径，它是新生成的String，复制了需要的字符；第二个值是 slice，指向 file_path 的一部分。
/// 
/// 返回的 String 本质上是把 file_path 中除了文件名之外的部分取出，连在 dir_name 后
/// 
/// 函数会过滤 "./" ，但不会过滤 "../"。
/// 如果支持后者，就可能需要处理 dir_name，这样开销更大也更复杂
fn split_path_and_file<'a>(dir_name: &str, file_path: &'a str) -> (String, &'a str) {
    let mut dir = String::from(dir_name);
    let mut pos = 0;
    loop {
        if let Some(new_pos) = (&file_path[pos..]).find('/') {
            // 这里用 new_pos == 1 是为了使用短路运算符，如果长度不是1就不需要做字符串比较了
            if new_pos > 1 || (new_pos == 1 && &file_path[pos..pos+new_pos] != ".") {
                // 加路径的时候要把 '/' 也加上
                dir += &file_path[pos..pos+new_pos+1];
            }
            pos += new_pos + 1;
        } else {
            break
        }
    }
    (dir, &file_path[pos..])
}

/// 获取文件所在路径。
fn get_file_dir<'a>(dir_name: &str, file_path: &'a str) -> String {
    split_path_and_file(dir_name, file_path).0
}


/// 打开目录。如果是根目录，特判直接返回 root；否则打开代表目录的 FsDir
/// 
/// 因为需要通过 move 传入 root，这个函数只在模块内使用。
/// 如果其他库需要打开目录(作为文件)，需要用 open_file 然后在 flags 里加入 DIR 一项
fn inner_open_dir(root: FsDir, dir_name: &str) -> Option<FsDir> {
    if dir_name == ROOT_DIR { 
        Some(root)
    } else {
        // 根目录是 "./" ，所以所有目录也是以 "./" 开头的，这里输入 fatfs 时要过滤掉这两个字符
        if let Ok(dir) = root.open_dir(&dir_name[2..]) {
            Some(dir)
        } else {
            return None
        }
    }
}

/// 在 dir_name 目录下，打开 name 文件。
/// 如果不包含 OpenFlags::DIR，可能出现如下情况：
/// 
/// 1. 文件存在，但要求创建 -> 清空文件并返回
/// 2. 文件存在，不要求创建 -> 直接返回文件
/// 3. 文件不存在，要求创建 -> 创建新文件并返回
/// 4. 文件不存在，不要求创建 -> 打开失败
/// 5. 文件不存在，但存在同名目录 -> 打开失败
/// 6. 其他情况，如路径不存在 -> 打开失败
/// 
/// 如果包含 OpenFlags::DIR，则只有打开已存在的目录成功时返回 FdDir
pub fn open_file(dir_name: &str, file_path: &str, flags: OpenFlags) -> Option<Arc<dyn File>> {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    let (real_dir, file_name) = split_path_and_file(dir_name, file_path);
    //println!("dir = {}, name = {}", real_dir, file_name);
    if let Some(dir) = inner_open_dir(root, real_dir.as_str()) {
        if flags.contains(OpenFlags::DIR) { // 要求打开目录
            // 用户传入 sys_open 的目录名如果是有斜线的，那么 file_path 就是空的了
            // 否则 file_path 是当前目录下的一个子目录的名字
            let dir = if file_name.len() == 0 { Ok(dir) } else { dir.open_dir(file_name) };
            match dir {
                Ok(dir) => {
                    // 不考虑是否有 CREATE 参数，只要找到目录就可以直接返回
                    Some(Arc::new(FdDir::new(String::from(real_dir) + file_name)))
                }
                // 如果找不到，也不考虑 CREATE。创建目录应该用 mkdir 而不是 open_file
                _ => { None }
            }
        } else { // 否则要求打开文件
            let (readable, writable) = flags.read_write();
            //println!("opened {}, {}", readable, writable);
            match dir.open_file(file_name) {
                Ok(file) => {
                    let fat_file = FatFile::new(readable, writable, get_file_dir(real_dir.as_str(), file_name), file);
                    if flags.contains(OpenFlags::CREATE) {
                        // 清空这个文件
                        fat_file.inner.lock().truncate();
                    };
                    Some(Arc::new(fat_file))
                },
                Err(Error::NotFound) => {
                    if flags.contains(OpenFlags::CREATE) {
                        let file = dir.create_file(file_name).unwrap();
                        Some(Arc::new(FatFile::new(readable, writable, get_file_dir(real_dir.as_str(), file_name), file)))
                    } else {
                        None
                    }
                },
                // 其他情况下(包括存在同名的目录的情况)，返回None
                _ => { None }
            }
        }
    } else {
        None
    }
}

/// 检查文件是否存在。
/// 如果目录本身不存在，那么也会返回 false，不会报错。
/// 
/// 这里并不直接试图打开文件检查是否成功，而是检查目录下是否存在对应文件。
/// 这是因为其他进程占用文件等情况也可能导致打开文件失败，所以打开失败不等于文件不存在
pub fn check_file_exists(dir_name: &str, file_path: &str) -> bool {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    let (real_dir, file_name) = split_path_and_file(dir_name, file_path);
    println!("check file exists: dir = {}, name = {}", real_dir, file_name);
    inner_open_dir(root, real_dir.as_str()).map(|dir| {
        for entry in dir.iter() {
            let file = entry.unwrap();
            if file.file_name() == file_name {
                return !file.is_dir();
            }
        }
        false
    }).map_or(false, |r| r)
}

/// 检查目录是否存在
pub fn check_dir_exists(dir_name: &str) -> bool {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    root.open_dir(dir_name).is_ok()
}



