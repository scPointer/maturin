//! FAT中的文件抽象。
//! 

#![deny(missing_docs)]

use lock::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use fatfs::{
    Read,
    Write,
    Seek,
    SeekFrom,
};

use super::File;
use super::FsFile;

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
    pub inner: Arc<Mutex<FsFile>>,
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
}

impl File for FatFile {
    /// 读取文件
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        if !self.readable {
            return None
        }
        let mut inner = self.inner.lock();
        let len = buf.len();
        let mut pos = 0;
        while pos < len {
            match inner.read(&mut buf[pos..]) {
                Ok(read_len) => {
                    if read_len == 0 {
                        break;
                    } else {
                        pos += read_len;
                    }
                }
                Err(_) => {
                    if pos == 0 { // 如果什么都没读到，则报错
                        return None
                    } else { //否则说明还是读了一些的
                        return Some(pos)
                    }
                }
            }
        };
        Some(pos)
    }
    /// 写入文件
    fn write(&self, buf: &[u8]) -> Option<usize> {
        if !self.writable {
            return None
        }
        let mut inner = self.inner.lock();
        let len = buf.len();
        let mut pos = 0;
        while pos < len {
            match inner.write(&buf[pos..]) {
                Ok(write_len) => {
                    if write_len == 0 {
                        break;
                    } else {
                        pos += write_len;
                    }
                }
                Err(_) => {
                    if pos == 0 {
                        return None
                    } else {
                        return Some(pos)
                    }
                }
            }
        };
        Some(pos)
    }
    /// 读取所有数据
    unsafe fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        // 获取文件大小
        let len = inner.seek(SeekFrom::End(0)).unwrap() as usize;
        inner.seek(SeekFrom::Start(0)).unwrap();
        let mut tmp: Vec<u8> = Vec::new();
        println!("file len {}=0x{:x}", len, len);
        tmp.resize(len, 0);
        let mut pos = 0;
        while pos < len {
            let read_len = inner.read(&mut tmp[pos..]).unwrap();
            //println!("read {} bytes", read_len);
            pos += read_len;
        }
        /*
        // println!("{} {} {} {}", tmp[0], tmp[1], tmp[2], tmp[3]); // elf
        println!("-------------------- test elf --------------------");
        let mut i: usize = 0x1000;
        while i < len && tmp[i] == 0 {
            i += 1;
        }
        print!("i = {} , tmp[i] = {}", i, tmp[i]);
        
        //for i in 0x1000..0x1010 {
        //    print!("{} ", tmp[i]);
        //}
        
        println!("");
        */
        tmp
    }
}