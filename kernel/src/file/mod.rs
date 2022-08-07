//! 文件类抽象，包含文件系统、stdin/stdout、管道等

mod device;
mod fd_manager;
mod fs_stat;
mod kstat;
mod pipe;
mod socket;
mod stdio;
mod vfs;

use crate::timer::TimeSpec;
use alloc::vec::Vec;

pub use fatfs::SeekFrom;

/// 文件类抽象
pub trait File: Send + Sync {
    /// 读文件内容到 buf，返回读到的字节数。
    /// 如文件不可读，返回 None。(相对应地，如果可读但没有读到内容，返回 Some(0))
    fn read(&self, buf: &mut [u8]) -> Option<usize>;
    /// 写 buf 中的内容到文件中，返回写入的字节数。
    /// 如文件不可写，返回 None。(相对应地，如果可写但无法继续写入内容，返回 Some(0))
    fn write(&self, buf: &[u8]) -> Option<usize>;
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
    fn set_time(&self, _atime: &TimeSpec, _mtime: &TimeSpec) -> bool {
        false
    }
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

pub use fd_manager::FdManager;
pub use pipe::Pipe;
pub use stdio::{Stderr, Stdin, Stdout};
pub use device::{
    check_dir_exists,
    check_file_exists,
    fs_init,
    get_kth_dir_entry_info_of_path,
    list_files_at_root,
    //load_testcases,
    load_next_testcase,
    mkdir,
    mount_fat_fs,
    open_file,
    origin_fs_stat,
    show_testcase_result,
    try_add_link,
    try_remove_link,
    umount_fat_fs,
};
pub use device::{FileDisc, OpenFlags};
pub use fs_stat::FsStat;
pub use kstat::normal_file_mode;
pub use kstat::{Kstat, StMode};
pub use socket::Socket;
pub use vfs::get_virt_file_if_possible;
