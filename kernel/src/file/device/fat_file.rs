//! FAT中的文件抽象。
//! 

//#![deny(missing_docs)]

use lock::Mutex;
use core::pin::Pin;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use fatfs::{
    Read,
    Write,
    Seek,
    SeekFrom,
};

use super::{File, FsFile};
use super::get_link_count;

use crate::file::{Kstat, StMode, normal_file_mode};
use crate::timer::{TimeSpec, UTIME_OMIT, UTIME_NOW};

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
    /// 文件名
    pub name: String,
    /// 时间信息
    pub tm: Mutex<FatFileTime>,
    /// 内部结构
    pub inner: Arc<Mutex<FsFile>>,
}

/// 文件的时间信息
pub struct FatFileTime {
    /// 最后一次访问时间
    pub atime: TimeSpec,
    /// 最后一次改变(modify)内容的时间
    pub mtime: TimeSpec,
    /// 最后一次改变(change)属性的时间
    pub ctime: TimeSpec,
}

impl FatFile {
    /// 构造一个带权限的 FatFile
    pub fn new(readable: bool, writable: bool, dir: String, name: String, fs_file: FsFile) -> Self {
        Self {
            readable: readable,
            writable: writable,
            dir: dir,
            name: name,
            inner: Arc::new(Mutex::new(fs_file)),
            tm: Mutex::new( FatFileTime {
                atime: TimeSpec::default(), // 目前创建时不从文件系统里拿时间，而是认为在系统启动时创建，
                mtime: TimeSpec::default(), // 因为 FAT 里的时间结构非常粗略，而且精度很低，
                ctime: TimeSpec::default(), // 不好适应实际操作中用到的秒/纳秒量级
            })
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
        info!("file len {}=0x{:x}", len, len);
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
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        let mut inner = self.inner.lock();
        let tm = self.tm.lock();
        let pos = 1;
        let pre_pos = inner.seek(SeekFrom::Current(0)).unwrap() as u64;
        let len = inner.seek(SeekFrom::End(0)).unwrap() as usize;
        inner.seek(SeekFrom::Start(pre_pos)).unwrap();
        let nlink = get_link_count(String::from(&self.dir[..]), self.name.as_str());
        unsafe {
            (*stat).st_dev = 1;
            (*stat).st_ino = 1;
            (*stat).st_nlink = nlink as u32;
            (*stat).st_mode = normal_file_mode(StMode::S_IFREG).bits();
            (*stat).st_size = len as u64;
            (*stat).st_uid = 0;
            (*stat).st_gid = 0;
            (*stat).st_atime_sec = tm.atime.tv_sec as isize;
            (*stat).st_atime_nsec = tm.atime.tv_nsec as isize;
            (*stat).st_mtime_sec = tm.mtime.tv_sec as isize;
            (*stat).st_mtime_nsec = tm.mtime.tv_nsec as isize;
            (*stat).st_ctime_sec = tm.ctime.tv_sec as isize;
            (*stat).st_ctime_nsec = tm.ctime.tv_nsec as isize;
        }
        true
    }
    /// 切换文件指针位置
    fn seek(&self, seekfrom: SeekFrom) -> Option<usize> {
        self.inner.lock().seek(seekfrom)
            .map(|pos| {
                if let SeekFrom::Start(origin) = seekfrom {
                    if origin > 0 {
                        return Some(origin as usize);
                    }
                }
                Some(pos as usize)
            }).unwrap_or_else(|_| {
            info!("seek {:#?}", seekfrom);
            if let SeekFrom::Start(pos) = seekfrom {
                if pos > 0 {
                    return Some(pos as usize);
                }
            }
            None
        })
    }
    /// 设置时间，返回是否设置成功。
    fn set_time(&self, atime: &TimeSpec, mtime: &TimeSpec) -> bool {
        let mut tm = self.tm.lock();
        tm.atime.set_as_utime(atime);
        tm.mtime.set_as_utime(mtime);
        true
    }
}