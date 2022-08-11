//! FAT中的文件抽象。
//!

//#![deny(missing_docs)]

use super::{get_link_count, File, FsFile, OpenFlags};
use crate::{
    file::{normal_file_mode, Kstat, StMode},
    timer::TimeSpec,
};
use alloc::{string::String, sync::Arc, vec::Vec};
use fatfs::{Read, Seek, SeekFrom, Write};
use lock::Mutex;

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
    /// 可变部分
    pub inner: Mutex<FatFileInnner>,
    /// 内部实际文件
    pub file: Arc<Mutex<FsFile>>,
}

/// 文件在os中运行时的可变信息
pub struct FatFileInnner {
    /// 最后一次访问时间
    pub atime: TimeSpec,
    /// 最后一次改变(modify)内容的时间
    pub mtime: TimeSpec,
    /// 最后一次改变(change)属性的时间
    pub ctime: TimeSpec,
    /// 打开时的选项。
    /// 主要用于判断 CLOEXEC，即 exec 时是否关闭。默认为 false。
    pub flags: OpenFlags,
}

impl FatFile {
    /// 构造一个带权限的 FatFile
    pub fn new(readable: bool, writable: bool, dir: String, name: String, fs_file: FsFile, flags: OpenFlags) -> Self {
        Self {
            readable: readable,
            writable: writable,
            dir: dir,
            name: name,
            file: Arc::new(Mutex::new(fs_file)),
            inner: Mutex::new(FatFileInnner {
                atime: TimeSpec::default(), // 目前创建时不从文件系统里拿时间，而是认为在系统启动时创建，
                mtime: TimeSpec::default(), // 因为 FAT 里的时间结构非常粗略，而且精度很低，
                ctime: TimeSpec::default(), // 不好适应实际操作中用到的秒/纳秒量级
                flags: flags,
            }),
        }
    }
}

impl File for FatFile {
    /// 读取文件
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        if !self.readable {
            return None;
        }
        let mut file = self.file.lock();
        let len = buf.len();
        let mut pos = 0;
        while pos < len {
            match file.read(&mut buf[pos..]) {
                Ok(read_len) => {
                    if read_len == 0 {
                        break;
                    } else {
                        pos += read_len;
                    }
                }
                Err(_) => {
                    if pos == 0 {
                        // 如果什么都没读到，则报错
                        return None;
                    } else {
                        //否则说明还是读了一些的
                        return Some(pos);
                    }
                }
            }
        }
        Some(pos)
    }
    /// 写入文件
    fn write(&self, buf: &[u8]) -> Option<usize> {
        if !self.writable {
            return None;
        }
        let mut file = self.file.lock();
        let len = buf.len();
        let mut pos = 0;
        while pos < len {
            match file.write(&buf[pos..]) {
                Ok(write_len) => {
                    if write_len == 0 {
                        break;
                    } else {
                        pos += write_len;
                    }
                }
                Err(_) => {
                    if pos == 0 {
                        return None;
                    } else {
                        return Some(pos);
                    }
                }
            }
        }
        Some(pos)
    }
    /// 读取所有数据
    unsafe fn read_all(&self) -> Vec<u8> {
        let mut file = self.file.lock();
        // 获取文件大小
        let len = file.seek(SeekFrom::End(0)).unwrap() as usize;
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut temp: Vec<u8> = Vec::new();
        info!("file len {}=0x{:x}", len, len);
        temp.resize(len, 0);
        let mut pos = 0;
        while pos < len {
            let read_len = file.read(&mut temp[pos..]).unwrap();
            //println!("read {} bytes", read_len);
            pos += read_len;
        }
        /*
        // println!("{} {} {} {}", temp[0], temp[1], temp[2], temp[3]); // elf
        println!("-------------------- test elf --------------------");
        let mut i: usize = 0x1000;
        while i < len && temp[i] == 0 {
            i += 1;
        }
        print!("i = {} , temp[i] = {}", i, temp[i]);

        //for i in 0x1000..0x1010 {
        //    print!("{} ", temp[i]);
        //}

        println!("");
        */
        temp
    }
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        let mut file = self.file.lock();
        let inner = self.inner.lock();
        let pre_pos = file.seek(SeekFrom::Current(0)).unwrap() as u64;
        let len = file.seek(SeekFrom::End(0)).unwrap() as usize;
        file.seek(SeekFrom::Start(pre_pos)).unwrap();
        let nlink = get_link_count(String::from(&self.dir[..]), self.name.as_str());
        unsafe {
            (*stat).st_dev = 1;
            (*stat).st_ino = 1;
            (*stat).st_nlink = nlink as u32;
            (*stat).st_mode = normal_file_mode(StMode::S_IFREG).bits();
            (*stat).st_size = len as u64;
            (*stat).st_uid = 0;
            (*stat).st_gid = 0;
            (*stat).st_atime_sec = inner.atime.tv_sec as isize;
            (*stat).st_atime_nsec = inner.atime.tv_nsec as isize;
            (*stat).st_mtime_sec = inner.mtime.tv_sec as isize;
            (*stat).st_mtime_nsec = inner.mtime.tv_nsec as isize;
            (*stat).st_ctime_sec = inner.ctime.tv_sec as isize;
            (*stat).st_ctime_nsec = inner.ctime.tv_nsec as isize;
        }
        true
    }
    /// 切换文件指针位置
    fn seek(&self, seekfrom: SeekFrom) -> Option<usize> {
        self.file
            .lock()
            .seek(seekfrom)
            .map(|pos| {
                if let SeekFrom::Start(origin) = seekfrom {
                    if origin > 0 {
                        return Some(origin as usize);
                    }
                }
                Some(pos as usize)
            })
            .unwrap_or_else(|_| {
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
        let mut inner = self.inner.lock();
        inner.atime.set_as_utime(atime);
        inner.mtime.set_as_utime(mtime);
        true
    }
    /// 设置文件状态信息，返回设置是否成功。
    fn set_status(&self, flags: OpenFlags) -> bool {
        self.inner.lock().flags = flags;
        true
    }
    /// 设置状态信息的 CLOEXEC 位，返回设置是否成功。
    /// 单独拆出接口是因为文件在 fd_manager 里存时是没有 mutex 锁的，
    /// 所以如果先 get 再 set 可能导致操作不原子。
    fn set_close_on_exec(&self, is_set: bool) -> bool {
        if is_set {
            self.inner.lock().flags |= OpenFlags::CLOEXEC;
        } else {
            self.inner.lock().flags &= !OpenFlags::CLOEXEC;
        }
        true
    }
    /// 获取文件状态信息
    fn get_status(&self) -> OpenFlags {
        self.inner.lock().flags
    }
    /// 清空文件
    fn clear(&self) {
        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.truncate().unwrap();
    }
}
