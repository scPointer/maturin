#![no_std]

extern crate alloc;
mod kstat;
mod open_flags;

use alloc::vec::Vec;
use core::any::Any;
use fatfs::SeekFrom;
pub use kstat::{Kstat, StMode, normal_file_mode};
pub use open_flags::OpenFlags;

/// 最大允许的文件描述符数量
pub const FD_LIMIT_HARD: usize = 256;

/// 文件类抽象
pub trait File: Send + Sync + AsAny {
    /// 读文件内容到 buf，返回读到的字节数。
    /// 如文件不可读，返回 None。(相对应地，如果可读但没有读到内容，返回 Some(0))
    fn read(&self, buf: &mut [u8]) -> Option<usize>;
    /// 写 buf 中的内容到文件中，返回写入的字节数。
    /// 如文件不可写，返回 None。(相对应地，如果可写但无法继续写入内容，返回 Some(0))
    fn write(&self, buf: &[u8]) -> Option<usize>;
    /// 从某个位置读文件内容到 buf 中，返回读到的字节数。如果文件不可读，返回 None。
    ///
    /// **需要支持 seek，但不改变指针位置；需要保证文件满足对同一个位置反复读/写是有效的，也即对于pipe、流等不适用**
    fn read_from_offset(&self, pos: usize, buf: &mut [u8]) -> Option<usize> {
        let old_pos = self.seek(SeekFrom::Current(0))?;
        let _ = self.seek(SeekFrom::Start(pos as u64))?;
        let read_len = self.read(buf);
        let _ = self.seek(SeekFrom::Current(old_pos as i64)).unwrap(); // 不管有没有读取，都要返回原来的位置
        read_len
    }
    /// 将 buf 写入文件中的某个位置，返回写入的字节数。如果文件不可写，返回 None。
    ///
    /// **需要支持 seek，但不改变指针位置；需要保证文件满足对同一个位置反复读/写是有效的，也即对于pipe、流等不适用**
    fn write_to_offset(&self, pos: usize, buf: &[u8]) -> Option<usize> {
        let old_pos = self.seek(SeekFrom::Current(0))?;
        let _ = self.seek(SeekFrom::Start(pos as u64))?;
        let write_len = self.write(buf);
        let _ = self.seek(SeekFrom::Current(old_pos as i64)).unwrap(); // 不管有没有写入，都要返回原来的位置
        write_len
    }
    /// 已准备好读。对于 pipe 来说，这意味着读端的buffer内有值
    fn ready_to_read(&self) -> bool {
        true
    }
    /// 已准备好写。对于 pipe 来说，这意味着写端的buffer未满
    fn ready_to_write(&self) -> bool {
        true
    }
    /// 是否已经终止。对于 pipe 来说，这意味着另一端已关闭
    fn is_hang_up(&self) -> bool {
        false
    }
    /// 处于“意外情况”。在 (p)select 和 (p)poll 中会使用到
    #[allow(unused)]
    fn in_exceptional_conditions(&self) -> bool {
        false
    }
    /// 清空文件
    fn clear(&self) {
    }
    /// 切换当前指针，返回切换后指针到文件开头的距离
    /// 如果文件本身不支持 seek(如pipe，是FIFO"设备") 则返回 None
    fn seek(&self, _seekfrom: SeekFrom) -> Option<usize> {
        None
    }
    /// 获取路径。
    /// - 专为 FsDir 设计。因为 linux 的 sys_openat 需要目录的文件描述符，但目录本身不能直接读写，所以特地开一个函数
    /// - 其他 File 类型返回 None 即可
    fn get_dir(&self) -> Option<&str> {
        None
    }
    /// 读取全部数据。
    /// 不是所有类型都实现了 read_all，目前只有文件系统中的文件是可知明确"大小"的，所以可以读"all"。
    /// 对于其他类型来说，这个函数没有实现。
    /// 调用者需要保证这个文件确实可以明确知道"大小"，所以它是 unsafe 的
    unsafe fn read_all(&self) -> Vec<u8> {
        unimplemented!();
    }
    /// 获取文件状态并写入 stat。成功时返回 true。
    ///
    /// 目前只有fat文件系统中的文件会处理这个函数
    fn get_stat(&self, _stat: *mut Kstat) -> bool {
        false
    }
    /// 设置时间，返回是否设置成功。
    ///
    /// 注意，格式需要考虑 crate::timer 模块中 UTIME_OMIT 和 UTIME_NOW 的特殊情况
    /* fn set_time(&self, _atime: &TimeSpec, _mtime: &TimeSpec) -> bool {
        false
    } */
    /// 设置文件状态信息，返回设置是否成功。
    fn set_status(&self, _flags: OpenFlags) -> bool {
        false
    }
    /// 设置状态信息的 CLOEXEC 位，返回设置是否成功。
    /// 单独拆出接口是因为文件在 fd_manager 里存时是没有 mutex 锁的，
    /// 所以如果先 get 再 set 可能导致操作不原子。
    fn set_close_on_exec(&self, _is_set: bool) -> bool {
        false
    }
    /// 获取文件状态信息
    fn get_status(&self) -> OpenFlags {
        OpenFlags::empty()
    }
    /// 发送消息，当且仅当这个文件是 socket 时可用
    fn sendto(&self, _buf: &[u8], _flags: i32, _dest_addr: usize) -> Option<usize> {
        None
    }
    /// 收取消息，当且仅当这个文件是 socket 时可用
    fn recvfrom(
        &self,
        _buf: &mut [u8],
        _flags: i32,
        _src_addr: usize,
        _src_len: &mut u32,
    ) -> Option<usize> {
        None
    }
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}
impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any { self }
}
