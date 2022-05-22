//! FAT文件系统设备的抽象
//! 包括读写文件等的支持

#![deny(missing_docs)]

use lazy_static::*;
use bitflags::*;
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

type FsIO = MemoryMappedFsIoType;
type FsTP = DefaultTimeProvider;
type FsOCC = LossyOemCpConverter;

type FsDir = fatfs::Dir<'static, FsIO, FsTP, FsOCC>;
type FsFile = fatfs::File<'static, FsIO, FsTP, FsOCC>;

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
        const CREATE = 1 << 6;
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

/// 把 FsFile 包装一层以适应 Trait File
pub struct FatFile {
    /// 是否可读
    pub readable: bool,
    /// 是否可写
    pub writable: bool,
    /// 所在文件夹的路径
    /// 
    /// 注意这里用 String 保存，而不是 &'static str之类的，
    /// 因为给出文件路径的可能是用户程序或者某个局部变量，如果不复制成 String，之后要用到的时候可能早已找不到了
    pub dir: String,
    /// 内部结构
    inner: Arc<Mutex<FsFile>>,
}

impl FatFile {
    /// 构造一个带权限的 FatFile
    pub fn new(readable: bool, writable: bool, dir: String, fs_file: FsFile) -> Self {
        Self {
            readable: readable,
            writable: writable,
            dir: dir,
            inner: Arc::new(Mutex::new(fs_file)),
        }
    }
    /// 读取所有数据
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        // 获取文件大小
        let len = inner.seek(SeekFrom::End(0)).unwrap() as usize;
        inner.seek(SeekFrom::Start(0)).unwrap();
        let mut tmp: Vec<u8> = Vec::new();
        tmp.resize(len, 0);
        inner.read(&mut tmp[..]);
        tmp
    }
}

impl File for FatFile {
    /// 读取文件
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        self.inner.lock().read(buf).ok()
    }
    /// 写入文件
    fn write(&self, buf: &[u8]) -> Option<usize> {
        self.inner.lock().write(buf).ok()
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

/// 在 dir_name 目录下，打开 name 文件。
/// 可能出现如下情况：
/// 
/// 1. 文件存在，但要求创建 -> 清空文件并返回
/// 2. 文件存在，不要求创建 -> 直接返回文件
/// 3. 文件不存在，要求创建 -> 创建新文件并返回
/// 4. 文件不存在，不要求创建 -> 打开失败
/// 5. 文件不存在，但存在同名目录 -> 打开失败
/// 6. 其他情况，如路径不存在 -> 打开失败
pub fn open_file(dir_name: &str, file_path: &str, flags: OpenFlags) -> Option<Arc<FatFile>> {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    root.open_dir(dir_name).map(|dir| {
        let (readable, writable) = flags.read_write();
        match dir.open_file(file_path) {
            Ok(file) => {
                let fat_file = FatFile::new(readable, writable, get_file_dir(dir_name, file_path), file);
                if flags.contains(OpenFlags::CREATE) {
                    // 清空这个文件
                    fat_file.inner.lock().truncate();
                };
                Some(Arc::new(fat_file))
            },
            Err(Error::NotFound) => {
                if flags.contains(OpenFlags::CREATE) {
                    let file = dir.create_file(file_path).unwrap();
                    Some(Arc::new(FatFile::new(readable, writable, get_file_dir(dir_name, file_path), file)))
                } else {
                    None
                }
            },
            // 其他情况下(包括存在同名的目录的情况)，返回None
            _ => {None}
        }
    }).map_or(None, |f| f)
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

    root.open_dir(real_dir.as_str()).map(|dir| {
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



