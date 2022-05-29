//! 与读写、文件相关的系统调用
//! 
//! 注意获取 current_task 的时候都使用了 unwrap()，这意味着默认只有用户程序才会调用 syscall 模块进行操作。
//! 如果内核态异常中断需要处理， trap 只能利用其他模块，如 MemorySet::handle_kernel_page_fault 等

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::string::String;
use lock::MutexGuard;

use crate::arch::{get_cpu_id};
use crate::arch::stdin::getchar;
use crate::task::{get_current_task};
use crate::task::TaskControlBlockInner;
use crate::utils::raw_ptr_to_ref_str;
use crate::file::{OpenFlags, Pipe};
use crate::file::{open_file, mkdir, check_dir_exists};
use crate::constants::{ROOT_DIR, AT_FDCWD};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// 获取当前工作路径
pub fn sys_getcwd(buf: *mut u8, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let dir = &tcb_inner.dir;
    // buf 可以塞下这个目录
    // 注意 + 1 是因为要塞 '\0'，- 1 是因为要去掉路径最开头的 '.'
    if dir.len() - 1 + 1 <= len {
        let slice = unsafe { core::slice::from_raw_parts_mut(buf, dir.len() - 1) };
        slice.copy_from_slice(&dir[1..].as_bytes());
        // 写入 '\0'
        unsafe { *buf.add(dir.len() - 1) = 0; }
        buf as isize
    } else { // 否则，buf 长度不够，但还是尽量把工作路径写进去
        if len - 1 > 0 {
            let slice = unsafe { core::slice::from_raw_parts_mut(buf, len - 1) };
            slice.copy_from_slice(&dir[1..len-1].as_bytes());
            unsafe { *buf.add(len - 1) = 0; }
        }
        0isize
    }
}

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
            //println!("[kernel] read syscall size {}", read_len);
            return read_len as isize
        }
    }
    -1
}

/// 写一个字串到 fd 代表的文件。这个串放在 buf 中，长为 len
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let pid = task.get_pid_num();
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

/// 从一个表示目录的文件描述符中获取目录名。
/// 如果这个文件描述符不是代表目录，则返回None
fn get_dir_from_fd(tcb_inner: &MutexGuard<TaskControlBlockInner>, dir_fd: i32) -> Option<String> {
    if dir_fd == AT_FDCWD { // 如果要求在当前路径下打开
        Some(String::from(tcb_inner.dir.as_str()))
    } else { // 否则需要去找 dir_fd 获取路径
        if let Ok(Some(dir)) = tcb_inner.fd_manager.get_file(dir_fd as usize).map(|f| {
                f.get_dir().map(|s| {String::from(s)}) }) {
            //println!("fd_dir = {}", dir);
            Some(dir)
        } else {
            None
        }
    }
}

/// 创建目录，成功时返回0，失败时返回 -1
/// 
/// - 如果path是相对路径，则它是相对于dirfd目录而言的。
/// - 如果path是绝对路径，则dirfd被忽略。
pub fn sys_mkdir(dir_fd: i32, path: *const u8, user_mode: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let file_path = unsafe { raw_ptr_to_ref_str(path) };
    let parent_dir = if file_path.starts_with("/") { // 绝对路径
        String::from("./") // 需要加上 '.'，因为 os 中约定根目录是以 '.' 开头
    } else { // 相对路径
        if let Some(dir) = get_dir_from_fd(&tcb_inner, dir_fd) {
            dir
        } else {
            return -1;
        }
    };
    if mkdir(parent_dir.as_str(), file_path) {
        0
    } else {
        -1
    }
}

/// 切换当前工作路径，**默认是相对路径**。切换成功时返回0，失败时返回-1
/// 
/// 会先检查要切换到的路径是否存在。
pub fn sys_chdir(path: *const u8) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let file_path = unsafe { raw_ptr_to_ref_str(path) };
    let current_path = &mut tcb_inner.dir;
    if !current_path.ends_with("/") { // 添加路径尾的斜杠
        *current_path += "/";
    }
    let new_path = current_path.clone() + file_path;
    if check_dir_exists(new_path.as_str()) {
        *current_path = new_path;
        0
    } else {
        -1
    }
}

/// 打开文件，返回对应的 fd。如打开失败，则返回 -1
pub fn sys_open(dir_fd: i32, path: *const u8, flags: u32, user_mode: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 目前认为所有用户程序都在根目录下，所以直接把路径当作文件名
    let file_name = unsafe { raw_ptr_to_ref_str(path) };
    // println!("fd = {}, file name = {}", dir_fd, file_name);
    // 因为 get_dir() 的所有权问题，这里只好用 String 暴力复制一遍了
    let dir = if let Some(dir) = get_dir_from_fd(&tcb_inner, dir_fd) {
        dir
    } else {
        return -1;
    };
    if let Some(node) = open_file(dir.as_str(), file_name, OpenFlags::from_bits(flags).unwrap()) {
        //println!("opened");
        if let Ok(fd) = tcb_inner.fd_manager.push(node) {
            //println!("return fd {}", fd);
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

/// 创建管道，在 *pipe 记录读管道的 fd，在 *(pipe+1) 记录写管道的 fd。
/// 成功时返回 0，失败则返回 -1
/// 
/// 注意，因为
pub fn sys_pipe(pipe: *mut u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let (pipe_read, pipe_write) = Pipe::new_pipe();
    if let Ok(fd1) = tcb_inner.fd_manager.push(Arc::new(pipe_read)) {
        if let Ok(fd2) = tcb_inner.fd_manager.push(Arc::new(pipe_write)) {
            unsafe {
                *pipe = fd1 as u32;
                *pipe.add(1) = fd2 as u32;
            }
            return 0
        } else {
            // 只成功插入了一个 fd。这种情况下要把 pipe_read 退出来
            tcb_inner.fd_manager.remove_file(fd1);
        }
    }
    -1
}

/// 复制一个 fd 中的文件到一个新 fd 中，成功时返回新的文件描述符，失败则返回 -1
pub fn sys_dup(fd: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Ok(new_fd) = tcb_inner.fd_manager.copy_fd_anywhere(fd) {
        new_fd as isize
    } else {
        -1
    }
}

/// 复制一个 fd 中的文件到指定的新 fd 中，成功时返回新的文件描述符，失败则返回 -1
pub fn sys_dup3(old_fd: usize, new_fd: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if tcb_inner.fd_manager.copy_fd_to(old_fd, new_fd) {
        new_fd as isize
    } else {
        -1
    }
}
