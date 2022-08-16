//! 内存中保存的虚文件，可以有 backend 指向它实际映射的内容

use alloc::{vec::Vec};
use lock::Mutex;
use crate::file::{File, Kstat, StMode, OpenFlags, SeekFrom, normal_file_mode};
use crate::memory::{Frame, addr_to_page_id, page_offset};
use crate::constants::PAGE_SIZE;

/// 临时文件
pub struct VirtFile {
    inner: Mutex<VirtFileInner>,
}

/// 实际保存的文件内容
pub struct VirtFileInner {
    /// 内部存储，保存分配的页帧
    frames: Vec<Frame>,
    /// 数据长度
    size: usize,
    /// 当前文件指针位置
    pos: usize,
    /// 打开时的选项。
    _flags: OpenFlags,
}

impl VirtFile {
    pub fn new(flags: OpenFlags) -> Self {
        Self {
            inner: Mutex::new(VirtFileInner::new(flags))
        }
    }
}

impl VirtFileInner {
    pub fn new(flags: OpenFlags) -> Self {
        Self {
            frames: Vec::new(),
            size: 0,
            pos: 0,
            _flags: flags,
        }
    }
    /// 内部对读文件的实现
    pub fn read_inner(&mut self, buf: &mut [u8]) -> Option<usize> {
        // 读到的实际长度
        let read_len = buf.len().min(self.size - self.pos);
        if read_len == 0 { // 如果已经没有内容可读了
            return Some(0);
        }
        // 现在读到文件内的第几页
        let page_now = addr_to_page_id(self.pos);
        // 页内的偏移量
        let off = page_offset(self.pos);
        // 如果一个页内可以解决，就简化处理
        if off + read_len <= PAGE_SIZE {
            buf[..read_len].copy_from_slice(&self.frames[page_now].as_slice()[off..off + read_len]);
            self.pos += read_len;
            return Some(read_len);
        }
        // 否则，需要每页分别处理
        // 先读第一页
        buf[..PAGE_SIZE - off].copy_from_slice(&self.frames[page_now].as_slice()[off..]);
        // 记录下现在读到 buf_pos 处
        let  mut buf_pos = PAGE_SIZE - off;
        // 然后从下一页开始，依次读整页
        for page in &self.frames[page_now+1..] {
            // 需要读完这一整页
            if buf_pos + PAGE_SIZE <= read_len {
                buf[buf_pos..buf_pos+PAGE_SIZE].copy_from_slice(page.as_slice());
                buf_pos += PAGE_SIZE;
            } else { // 否则，是最后一页了。这里不用考虑文件长度，因为计算 read_len 的时候已保证了不会超长度
                buf[buf_pos..read_len].copy_from_slice(&page.as_slice()[..read_len - buf_pos]);
                //buf_pos += read_len - PAGE_SIZE;
                break;
            }
        }
        self.pos += read_len;
        Some(read_len)
    }
    /// 内部对写文件的实现
    pub fn write_inner(&mut self, buf: &[u8]) -> Option<usize> {
        if buf.len() == 0 { // 特判没有实际写入的情况
            return Some(0);
        }
        if (self.size & (0x100_0000 - 1)) == 0 {
            //println!("write {} size {}", buf.len(), self.size);
        }
        // 需要用到多少页才够
        let page_needed = addr_to_page_id(self.pos + buf.len() - 1);
        // seek 是可以搜到文件实际末尾之后的，所以此时写入的时候要提前申请之前空间
        for _i in self.frames.len()..=page_needed {
            // 此处不处理申请时溢出的情况
            self.frames.push(Frame::new().unwrap());
        };
        // 现在在文件内的第几页
        let page_now = addr_to_page_id(self.pos);
        // 页内的偏移量
        let off = page_offset(self.pos);
        // 如果一个页内可以解决，就简化处理
        if off + buf.len() <= PAGE_SIZE {
            self.frames[page_now].as_slice_mut()[off..off+buf.len()].copy_from_slice(buf);
            // 更新文件指针和文件大小
            self.pos += buf.len();
            self.size = self.size.max(self.pos);
            return Some(buf.len());
        }
        // 否则先读第一页
        self.frames[page_now].as_slice_mut()[off..].copy_from_slice(&buf[..PAGE_SIZE - off]);
        // 记录下现在写到 buf_pos 处
        let mut buf_pos = PAGE_SIZE - off;
        // 然后从下一页开始，依次读整页
        for page in &mut self.frames[page_now+1..] {
            // 需要读完这一整页
            if buf_pos + PAGE_SIZE <= buf.len() {
                page.as_slice_mut().copy_from_slice(&buf[buf_pos..buf_pos+PAGE_SIZE]);
                buf_pos += PAGE_SIZE;
            } else { //否则，是最后一页了。这里不用考虑文件长度，因为计算 read_len 的时候已保证了不会超长度
                page.as_slice_mut()[..buf.len() - buf_pos].copy_from_slice(&buf[buf_pos..]);
                //buf_pos += buf.len() - PAGE_SIZE;
                break;
            }
        }
        // 更新文件指针和文件大小
        self.pos += buf.len();
        self.size = self.size.max(self.pos);
        Some(buf.len())
    }
    /// 从某个位置读文件内容到 buf 中，返回读到的字节数，但不改变指针位置
    fn read_from_offset(&mut self, pos: usize, buf: &mut [u8]) -> Option<usize> {
        let old_pos = self.pos;
        self.pos = pos;
        let read_len = self.read_inner(buf);
        self.pos = old_pos;
        read_len
    }
    /// 将 buf 写入文件中的某个位置，返回读到的字节数，但不改变指针位置
    fn write_to_offset(&mut self, pos: usize, buf: &[u8]) -> Option<usize> {
        let old_pos = self.pos;
        self.pos = pos;
        let write_len = self.write_inner(buf);
        self.pos = old_pos;
        write_len
    }
    /// 读取文件大小
    pub fn get_size(&self) -> usize {
        self.size
    }
    /// 修改文件指针位置
    pub fn seek(&mut self, seekfrom: SeekFrom) -> Option<usize> {
        match seekfrom {
            SeekFrom::Start(pos) => {
                self.pos = pos as usize;
            }
            SeekFrom::Current(pos) => {
                if self.pos as i64 + pos < 0 { // 不能移动到文件前
                    return None;
                } else {
                    self.pos = (self.pos as i64 + pos) as usize;
                }
            }
            SeekFrom::End(pos) => {
                if self.size as i64 + pos < 0 {
                    return None;
                } else {
                    self.pos = (self.size as i64 + pos) as usize;
                }
            }
        };
        Some(self.pos)
    }
    /// 设置位置。其实和 seek 相同，但是更简化，且不做检查
    pub unsafe fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }
    /// 清空文件
    fn clear(&mut self) {
        self.pos = 0;
        self.size = 0;
        self.frames.clear();
    }
}

impl File for VirtFile {
    /// 读取文件
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        self.inner.lock().read_inner(buf)
    }
    /// 写入文件
    fn write(&self, buf: &[u8]) -> Option<usize> {
        self.inner.lock().write_inner(buf)
    }
    /// 文件属性
    fn get_stat(&self, stat: *mut Kstat) -> bool {
        //println!("vfile stat");
        let inner = self.inner.lock();
        unsafe {
            (*stat).st_dev = 0;
            (*stat).st_ino = 0;
            (*stat).st_nlink = 1;
            (*stat).st_mode = normal_file_mode(StMode::S_IFREG).bits();
            (*stat).st_size = inner.get_size() as u64;
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
    /// 切换文件指针位置
    fn seek(&self, seekfrom: SeekFrom) -> Option<usize> {
        self.inner.lock().seek(seekfrom)
    }
    /// 清空文件
    fn clear(&self) {
        self.inner.lock().clear();
    }
    /// 从某个位置读文件内容到 buf 中，返回读到的字节数，但不改变指针位置
    fn read_from_offset(&self, pos: usize, buf: &mut [u8]) -> Option<usize> {
        self.inner.lock().read_from_offset(pos, buf)
    }
    /// 将 buf 写入文件中的某个位置，返回读到的字节数，但不改变指针位置
    fn write_to_offset(&self, pos: usize, buf: &[u8]) -> Option<usize> {
        self.inner.lock().write_to_offset(pos, buf)
    }
}