//! 与读写、文件相关的系统调用
//! 
//! 注意获取 current_task 的时候都使用了 unwrap()，这意味着默认只有用户程序才会调用 syscall 模块进行操作。
//! 如果内核态异常中断需要处理， trap 只能利用其他模块，如 MemorySet::handle_kernel_page_fault 等

#![deny(missing_docs)]

use alloc::sync::Arc;

use crate::arch::{get_cpu_id};
use crate::arch::stdin::getchar;
use crate::task::{get_current_task};
use crate::utils::raw_ptr_to_ref_str;
use crate::file::{OpenFlags, open_file, Pipe};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// 从 fd 代表的文件中读一个字串，最长为 len，放入 buf 中
pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };

    // 尝试了一下用 .map 串来写，但实际效果好像不如直接 if... 好看
    if let Ok(file) = tcb_inner.fd_manager.get_file(fd) {
        // 读文件可能触发进程切换
        drop(tcb_inner);
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
        // 写文件也可能触发进程切换
        drop(tcb_inner);
        if let Some(write_len) = file.write(slice) {
            return write_len as isize
        }
    }
    -1
}

/// 打开文件，返回对应的 fd。如打开失败，则返回 -1
pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 目前认为所有用户程序都在根目录下，所以直接把路径当作文件名
    let file_name = unsafe { raw_ptr_to_ref_str(path) }; 
    if let Some(node) = open_file(file_name, OpenFlags::from_bits(flags).unwrap()) {
        if let Ok(fd) = tcb_inner.fd_manager.push(node) {
            return fd as isize
        }
    }
    -1
}

/// 关闭文件，成功时返回 0，失败时返回 -1
pub fn sys_close(fd: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Ok(file) = tcb_inner.fd_manager.remove_file(fd) {
        // 其实可以对 file 做最后处理。
        // 但此处不知道 file 的具体类型，所以还是推荐实现 Trait File 的类型自己写 Drop 时处理
        0
    } else {
        -1
    }
}

/// 创建管道，在 *pipe 记录读管道的 fd，在 *(pipe+1) 记录写管道的 fd
/// 成功时返回 0，失败则返回 -1
pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let (pipe_read, pipe_write) = Pipe::new_pipe();
    if let Ok(fd1) = tcb_inner.fd_manager.push(Arc::new(pipe_read)) {
        if let Ok(fd2) = tcb_inner.fd_manager.push(Arc::new(pipe_write)) {
            unsafe {
                *pipe = fd1;
                *pipe.add(1) = fd2;
            }
            return 0
        } else {
            // 只成功插入了一个 fd。这种情况下要把 pipe_read 退出来
            tcb_inner.fd_manager.remove_file(fd1);
        }
    }
    -1
}

/// 复制一个 fd 中的文件到一个新 fd 中，成功时返回 0，失败则返回 -1
pub fn sys_dup(fd: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Ok(new_fd) = tcb_inner.fd_manager.copy_fd(fd) {
        new_fd as isize
    } else {
        -1
    }
}
