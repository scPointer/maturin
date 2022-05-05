//! 与读写、文件相关的系统调用

#![deny(missing_docs)]

use crate::arch::{get_cpu_id};
use crate::arch::stdin::getchar;
use crate::task::{get_current_task};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// 从 fd 代表的文件中读一个字串，最长为 len，放入 buf 中
pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    if let Ok(file) = tcb_inner.fd_manager.get_file(fd) {
        if let Some(read_len) = file.read(slice) {
            return read_len as isize
        }
    }
    -1
}

/// 写一个字串到 fd 代表的文件。这个串放在 buf 中，长为 len
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();

    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
    if let Ok(file) = tcb_inner.fd_manager.get_file(fd) {
        if let Some(write_len) = file.write(slice) {
            return write_len as isize
        }
    }
    -1
}
