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
use crate::file::{OpenFlags, Pipe, Kstat};
use crate::file::{
    open_file, 
    mkdir, 
    check_dir_exists,
    try_add_link,
    try_remove_link,
    umount_fat_fs,
    mount_fat_fs,
    get_kth_dir_entry_info_of_path,
};
use crate::constants::{ROOT_DIR, AT_FDCWD, DIR_ENTRY_SIZE};

use super::{Dirent64, Dirent64_Type, OpenatError, IoVec};

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
            //println!("[kernel] read syscall size {} wanted {}", read_len, len);
            return read_len as isize
        }
    }
    -1
}

/// 写一个字串到 fd 代表的文件。这个串放在 buf 中，长为 len
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    //println!("write pos {:x}", buf as usize);
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

/// 从同一个 fd 中读取一组字符串。
/// 目前这个 syscall 借用 sys_read 来实现
pub fn sys_readv(fd: usize, iov: *mut IoVec, iov_cnt: usize) -> isize {
    if iov_cnt < 0 {
        return -1;
    }
    info!("readv fd {}", fd);
    let mut read_len = 0;
    for i in 0..iov_cnt {
        let io_vec: &IoVec = unsafe { &*iov.add(i) };
        println!("sys_readv: io_vec.base {:x}, len {:x}", io_vec.base as usize, io_vec.len);
        let ret = sys_read(fd, io_vec.base, io_vec.len);
        if ret == -1 {
            break
        } else {
            read_len += ret;
        }
    }
    read_len
}

/// 写入一组字符串到同一个 fd 中。
/// 目前这个 syscall 借用 sys_write 来实现
pub fn sys_writev(fd: usize, iov: *const IoVec, iov_cnt: usize) -> isize {
    if iov_cnt < 0 {
        return -1;
    }
    info!("writev fd {}", fd);
    let mut written_len = 0;
    for i in 0..iov_cnt {
        let io_vec: &IoVec = unsafe { &*iov.add(i) };
        println!("sys_writev: io_vec.base {:x}, len {:x}", io_vec.base as usize, io_vec.len);
        let ret = sys_write(fd, io_vec.base, io_vec.len);
        if ret == -1 {
            break
        } else {
            written_len += ret;
        }
    }
    written_len
}

/// 获取文件状态信息
pub fn sys_fstat(fd: usize, kstat: *mut Kstat) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();

    if let Ok(file) = tcb_inner.fd_manager.get_file(fd) {
        if file.get_stat(kstat) {
            return 0;
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

/// 从输入的路径文件描述符和文件名，解析实际的父目录和文件名。
/// 成功时返回 0，失败时返回 -1
/// 
/// 适用于 open/madir/link/unlink 等
fn resolve_path_from_fd<'a>(tcb_inner: &MutexGuard<TaskControlBlockInner>, dir_fd: i32, path: *const u8) -> Option<(String, &'a str)>{
    let file_path = unsafe { raw_ptr_to_ref_str(path) };
    //println!("file_path {}", file_path);
    if file_path.starts_with("/") { // 绝对路径
        Some((String::from("./"), &file_path[1..])) // 需要加上 '.'，因为 os 中约定根目录是以 '.' 开头
    } else { // 相对路径
        if let Some(dir) = get_dir_from_fd(&tcb_inner, dir_fd) {
            Some((dir, file_path))
        } else {
            return None;
        }
    }
}

/// 创建硬链接。成功时返回0，失败时返回-1
pub fn sys_linkat(old_dir_fd: i32, old_path: *const u8, new_dir_fd: i32, new_path: *const u8, flags: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Some((old_path, old_file)) = resolve_path_from_fd(&tcb_inner, old_dir_fd, old_path) {
        if let Some((new_path, new_file)) = resolve_path_from_fd(&tcb_inner, new_dir_fd, new_path) {
            if try_add_link(old_path, old_file, new_path, new_file) {
                return 0;
            }
        }
    }
    -1
}

/// 删除硬链接，并在链接数为0时实际删除文件。成功时返回0，失败时返回-1
pub fn sys_unlinkat(dir_fd: i32, path: *const u8, flags: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Some((path, file)) = resolve_path_from_fd(&tcb_inner, dir_fd,path) {
        if try_remove_link(path, file){
            return 0;
        }
    }
    -1
}

/// 挂载文件系统。成功时返回0，失败时返回-1。
/// 
/// 目前只是语义上实现，还没有真实板子上测试过
pub fn sys_mount(device: *const u8, mount_path: *const u8, fs_type: *const u8, flags: u32, data: *const u8) -> isize {
    let fs_type = unsafe { raw_ptr_to_ref_str(fs_type) };
    if fs_type != "vfat" { // 不支持挂载其他类型
        return -1;
    }
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 这里把 fd 写成"当前目录"，但其实如果内部发现路径是 '/' 开头，会用绝对路径替代。
    // 这也是其他类似调用 open/close/mkdir/linkat 等的逻辑
    if let Some((device_path, device_file)) = resolve_path_from_fd(&tcb_inner, AT_FDCWD, device) {
        if let Some((mut mount_path, mount_file)) = resolve_path_from_fd(&tcb_inner, AT_FDCWD, mount_path) {
            mount_path += mount_file;
            if !mount_path.ends_with('/') { // 挂载到的是一个目录，但用户输入目录时不一定加了 '/'
                mount_path.push('/');
            }
            if mount_fat_fs(device_path, device_file, mount_path) {
                return 0;
            }
        }
    }
    -1
}

/// 卸载文件系统。成功时返回0，失败时(目录不存在/未挂载等)返回-1。
/// 
/// 目前只是语义上实现，还没有真实板子上测试过
pub fn sys_umount(mount_path: *const u8, flags: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Some((mut mount_path, mount_file)) = resolve_path_from_fd(&tcb_inner, AT_FDCWD, mount_path) {
        mount_path += mount_file;
        if !mount_path.ends_with('/') { 
            mount_path.push('/');
        }
        if umount_fat_fs(mount_path) {
            return 0;
        }
    }
    -1
}

/// 创建目录，成功时返回 0，失败时返回 -1
/// 
/// - 如果path是相对路径，则它是相对于dirfd目录而言的。
/// - 如果path是绝对路径，则dirfd被忽略。
pub fn sys_mkdir(dir_fd: i32, path: *const u8, user_mode: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Some((parent_dir, file_path)) = resolve_path_from_fd(&tcb_inner, dir_fd, path) {
        if mkdir(parent_dir.as_str(), file_path) {
            return 0;
        }
    }
    -1
}

/// 切换当前工作路径，如果以.开头，默认是相对路径；如果以/开头，默认是绝对路径。切换成功时返回0，失败时返回-1
/// 
/// 会先检查要切换到的路径是否存在。
pub fn sys_chdir(path: *const u8) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let file_path = unsafe { raw_ptr_to_ref_str(path) };
    
    let new_path = {
        if file_path.starts_with("/") {
            String::from(".") + file_path
        } else {
            let current_path = &mut tcb_inner.dir;
            if !current_path.ends_with("/") { // 添加路径尾的斜杠
            *current_path += "/";
            }
            current_path.clone() + file_path
        }
    };
    //info!("new path = {}", new_path);
    if check_dir_exists(new_path.as_str()) {
        *(&mut tcb_inner.dir) = new_path;
        0
    } else {
        -1
    }
}

/// 打开文件，返回对应的 fd。如打开失败，则返回 -1
pub fn sys_open(dir_fd: i32, path: *const u8, flags: u32, user_mode: u32) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 如果 fd 已满，则不再添加
    if tcb_inner.fd_manager.is_full() {
        return OpenatError::EMFILE as isize;
    }
    if let Some((parent_dir, file_path)) = resolve_path_from_fd(&tcb_inner, dir_fd, path) {
        let mut file_path = String::from(file_path);
        // 特判当前目录。
        // 根据测例文档描述，一般有3种情况
        // 1. '/' 开头的绝对路径，如 /dev
        // 2. './' 开头的相对路径，如 /abc
        // 3. 字母数字开头的相对路径，如 def.txt
        // 而把路径直接写成当前目录的情况比较特殊，不包含在以上三种之内
        //println!("file path = {}, len = {}, flags = {:x}", file_path, file_path.len(), flags);
        if file_path == "." {
            file_path.push('/');
        } else if file_path.starts_with(".//") {
            file_path.remove(1);
        }
        println!("try open parent_dir={} file_path={} flag={:x}", parent_dir, file_path, flags);
        if let Some(open_flags) = OpenFlags::from_bits(flags) {
            println!("[{:#?}]", open_flags);
            //println!("opened");
            if let Some(node) = open_file(parent_dir.as_str(), file_path.as_str(), open_flags) {
                //println!("opened");
                if let Ok(fd) = tcb_inner.fd_manager.push(node) {
                    println!("return fd {}", fd);
                    return fd as isize
                } else if open_flags.contains(OpenFlags::EXCL) {
                    // 要求创建文件却打开失败，说明是文件已存在
                    return OpenatError::EEXIST as isize
                }
            }
        }
    }
    OpenatError::ENOENT as isize
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
        OpenatError::EMFILE as isize
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

pub fn sys_getdents64(fd: usize, buf: *mut Dirent64, len: usize) -> isize {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    if let Some(dir) = get_dir_from_fd(&tcb_inner, fd as i32) {
        let entry_id = unsafe { (*buf).d_off as usize / DIR_ENTRY_SIZE };
        if let Some((is_dir, file_name)) = get_kth_dir_entry_info_of_path(dir.as_str(), entry_id) {
            unsafe {
                (*buf).d_ino = 1;
                (*buf).d_off += DIR_ENTRY_SIZE as i64;
                (*buf).d_reclen = DIR_ENTRY_SIZE as u16;
                (*buf).set_type(if is_dir { Dirent64_Type::DIR } else { Dirent64_Type::REG });
                let name_start = (buf as usize + (*buf).d_name_offset()) as *mut u8;
                // 算出还能放 d_name 的位置。其中字符串结尾加一个0保证最后不溢出
                let len  = len - (*buf).d_name_offset() - 1;
                let copy_len = if len < file_name.len() { len } else { file_name.len() };
                let slice = unsafe { core::slice::from_raw_parts_mut(name_start, copy_len) };
                slice.copy_from_slice(&file_name.as_bytes());
                // 字符串结尾
                *name_start.add(copy_len) = 0;
                return (copy_len + (*buf).d_name_offset()) as isize;
            }
        }
    }
    -1
}