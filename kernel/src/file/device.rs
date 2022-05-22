use lazy_static::*;
use lock::Mutex;
use fatfs::{
    FsOptions,
    FileSystem,
    DefaultTimeProvider, 
    LossyOemCpConverter,
};

use crate::drivers::{new_memory_mapped_fs, MemoryMappedFsIoType};

lazy_static! {
    static ref MEMORY_FS: Mutex<FileSystem<MemoryMappedFsIoType, DefaultTimeProvider, LossyOemCpConverter>> = Mutex::new(new_memory_mapped_fs());
}

/// 输出根目录下的所有文件
pub fn list_files_at_root() {
    let fs = MEMORY_FS.lock();
    let root = fs.root_dir();

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

