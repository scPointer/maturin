pub mod fsio {
    pub use fscommon::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
}

pub struct MemoryMappedDevice {
    start: usize,
    end: usize,
    pos: usize,
}

impl MemoryMappedDevice {
    /// 初始化设备。注意设置 pos = start，而不是 pos=0
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start,
            end: end,
            pos: start,
        }
    }
}

impl fsio::Read for MemoryMappedDevice {
    fn read(&mut self, buf: &mut [u8]) -> fsio::Result<usize> {
        let read_len = core::cmp::min(self.end - self.pos, buf.len());
        let data = unsafe { core::slice::from_raw_parts(self.pos as *const u8, read_len) };
        if read_len == buf.len() {
            buf.copy_from_slice(data);
        } else {
            (&mut buf[..read_len]).copy_from_slice(data);
        }
        self.pos += read_len;
        //println!("read len {}, pos {:x}", read_len, self.pos);
        Ok(read_len)
    }
}

impl fsio::Write for MemoryMappedDevice {
    fn write(&mut self, buf: &[u8]) -> fsio::Result<usize> {
        let write_len = core::cmp::min(self.end - self.pos, buf.len());
        let write_to = unsafe { core::slice::from_raw_parts_mut(self.pos as *mut u8, write_len) };
        if write_len == buf.len() {
            write_to.copy_from_slice(buf);
        } else {
            write_to.copy_from_slice(&buf[..write_len]);
        }
        self.pos += write_len;
        Ok(write_len)
    }
    /// 实际上没有 flush 。因为文件系统是从qemu引入或者 .data 段拿过来的。
    ///
    /// 1. 对于前者，write 完成的时候就已经写回，buf 在上层的 BufStream里
    /// 2. 对于后者，整个fs都在内存里，不存在一个需要"写回"到磁盘的文件
    fn flush(&mut self) -> fsio::Result<()> {
        Ok(())
    }
}

impl fsio::Seek for MemoryMappedDevice {
    fn seek(&mut self, pos: fsio::SeekFrom) -> fsio::Result<u64> {
        let new_pos = match pos {
            fsio::SeekFrom::Current(delta) => (self.pos as i64 + delta) as usize,
            fsio::SeekFrom::Start(delta) => (self.start as u64 + delta) as usize,
            fsio::SeekFrom::End(delta) => (self.end as i64 + delta) as usize,
        };
        // 对于一般的文件来说，pos 在 self.end 后面时会自动扩展文件大小，不会报错
        // 但是此处面对的是文件系统镜像，它的大小不会增长，所以需要直接报错
        if new_pos < self.start || new_pos > self.end {
            Err(fsio::Error::from(fsio::ErrorKind::Uncategorized))
        } else {
            self.pos = new_pos;
            Ok((self.pos - self.start) as u64)
        }
    }
}
