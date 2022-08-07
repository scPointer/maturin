//! 与读写、文件相关的系统调用
//!
//! 注意获取 current_task 的时候都使用了 unwrap()，这意味着默认只有用户程序才会调用 syscall 模块进行操作。
//! 如果内核态异常中断需要处理， trap 只能利用其他模块，如 MemorySet::handle_kernel_page_fault 等

//#![deny(missing_docs)]

use super::{
    Dirent64, Dirent64Type, SysResult, ErrorNo, IoVec, UtimensatFlags, Fcntl64Cmd, SEEK_CUR,
    SEEK_END, SEEK_SET,
};
use crate::{
    constants::{AT_FDCWD, DIR_ENTRY_SIZE, SENDFILE_BUFFER_SIZE},
    file::{
        check_dir_exists, check_file_exists, get_kth_dir_entry_info_of_path, mkdir, mount_fat_fs,
        open_file, origin_fs_stat, try_add_link, try_remove_link, umount_fat_fs,
    },
    file::{FsStat, Kstat, OpenFlags, Pipe, SeekFrom},
    task::{get_current_task, TaskControlBlock},
    timer::TimeSpec,
    utils::raw_ptr_to_ref_str,
};
use alloc::{string::String, sync::Arc};

/// 获取当前工作路径
pub fn sys_getcwd(buf: *mut u8, len: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let tcb_inner = task.inner.lock();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_page(buf as usize).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }
    let dir = &tcb_inner.dir;
    // buf 可以塞下这个目录
    // 注意 + 1 是因为要塞 '\0'，- 1 是因为要去掉路径最开头的 '.'
    if dir.len() - 1 + 1 <= len {
        let slice = unsafe { core::slice::from_raw_parts_mut(buf, dir.len() - 1) };
        //info!("buf at {:x}, len {}, slice len {}", buf as usize, len, slice.len());
        slice.copy_from_slice(&dir[1..].as_bytes());
        // 写入 '\0'
        unsafe {
            *buf.add(dir.len() - 1) = 0;
        }
        Ok(buf as usize)
    } else {
        // 否则，buf 长度不够，按照规范返回 ERANGE
        Err(ErrorNo::ERANGE)
        /*
        if len - 1 > 0 {
            let slice = unsafe { core::slice::from_raw_parts_mut(buf, len - 1) };
            slice.copy_from_slice(&dir[1..len-1].as_bytes());
            unsafe { *buf.add(len - 1) = 0; }
        }
        Ok(0)
        */
    }
}

/// 从 fd 代表的文件中读一个字串，最长为 len，放入 buf 中
pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let tcb_inner = task.inner.lock();
    let mut task_vm = task.vm.lock();
    info!("sys_read fd {} buf {:x} len {}", fd, buf as usize, len);
    /*
    let buf = buf as usize;
    let buf = match UserPtr::try_from((buf, &mut task_vm)) {
        Ok(buf) => buf,
        Err(num) => {
            return num as isize;
        }
    };
    */
    if task_vm.manually_alloc_page(buf as usize).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    // 尝试了一下用 .map 串来写，但实际效果好像不如直接 if... 好看
    if let Ok(file) = task.fd_manager.lock().get_file(fd) {
        //let pos = file.seek(SeekFrom::Current(0)).unwrap();
        //info!("read from pos {pos}");
        // 读文件可能触发进程切换
        drop(tcb_inner);
        drop(task_vm);
        if let Some(read_len) = file.read(slice) {
            //println!("[kernel] read syscall size {} wanted {}", read_len, len);
            return Ok(read_len);
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 写一个字串到 fd 代表的文件。这个串放在 buf 中，长为 len
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let tcb_inner = task.inner.lock();
    let task_vm = task.vm.lock();
    //println!("write pos {:x}", buf as usize);
    /*
    let buf = buf as usize;
    let buf = match UserPtr::try_from((buf, &mut task_vm)) {
        Ok(buf) => buf,
        Err(num) => {
            return num as isize;
        }
    };
    */
    let slice = unsafe { core::slice::from_raw_parts(buf, len) };

    if let Ok(file) = task.fd_manager.lock().get_file(fd) {
        // 写文件也可能触发进程切换
        drop(tcb_inner);
        drop(task_vm);
        if let Some(write_len) = file.write(slice) {
            return Ok(write_len);
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 从同一个 fd 中读取一组字符串。
/// 目前这个 syscall 借用 sys_read 来实现
pub fn sys_readv(fd: usize, iov: *mut IoVec, iov_cnt: usize) -> SysResult {
    info!("sys_readv fd {}", fd);
    let mut read_len = 0;
    for i in 0..iov_cnt {
        let io_vec: &IoVec = unsafe { &*iov.add(i) };
        info!(
            "sys_readv: io_vec.base {:x}, len {:x}",
            io_vec.base as usize, io_vec.len
        );
        match sys_read(fd, io_vec.base, io_vec.len) {
            Ok(len) => read_len += len,
            Err(_) => { break; },
        }
    }
    Ok(read_len)
}

/// 写入一组字符串到同一个 fd 中。
/// 目前这个 syscall 借用 sys_write 来实现
pub fn sys_writev(fd: usize, iov: *const IoVec, iov_cnt: usize) -> SysResult {
    info!("sys_writev fd {}", fd);
    let mut written_len = 0;
    for i in 0..iov_cnt {
        let io_vec: &IoVec = unsafe { &*iov.add(i) };
        info!(
            "sys_writev: io_vec.base {:x}, len {:x}",
            io_vec.base as usize, io_vec.len
        );
        match sys_write(fd, io_vec.base, io_vec.len) {
            Ok(len) =>  written_len += len,
            Err(_) => { break; }
        }
    }
    Ok(written_len)
}

/// 在 offset 位置读 count 个字符。这个文件必须支持 seek
pub fn sys_pread(fd: usize, buf: *mut u8, count: usize, offset: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let tcb_inner = task.inner.lock();
    let mut task_vm = task.vm.lock();
    info!("sys_pread fd {} buf {:x} count {} offset {}", fd, buf as usize, count, offset);
    if task_vm.manually_alloc_page(buf as usize).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, count) };
    // 尝试了一下用 .map 串来写，但实际效果好像不如直接 if... 好看
    if let Ok(file) = task.fd_manager.lock().get_file(fd) {
        if let Some(pos) = file.seek(SeekFrom::Start(offset as u64)) {
            // 保证确实 seek 到对应位置。而不是超过文件末尾
            if pos == offset {
                // 读文件可能触发进程切换
                drop(tcb_inner);
                drop(task_vm);
                if let Some(read_len) = file.read(slice) {
                    //println!("[kernel] read syscall size {} wanted {}", read_len, len);
                    return Ok(read_len);
                }
            }
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 获取文件状态信息
pub fn sys_fstat(fd: usize, kstat: *mut Kstat) -> SysResult {
    let task = get_current_task().unwrap();
    if let Ok(file) = task.fd_manager.lock().get_file(fd) {
        if file.get_stat(kstat) {
            return Ok(0);
        }
    }
    Err(ErrorNo::EINVAL)
}
/// 获取文件状态信息，但是给出的是目录 fd 和相对路径。
pub fn sys_fstatat(dir_fd: i32, path: *const u8, kstat: *mut Kstat) -> SysResult {
    let task = get_current_task().unwrap();
    if let Some((path, file)) = resolve_path_from_fd(&task, dir_fd, path) {
        info!("fstatat: path {} file {}", path, file);
        // 打开文件，选项为空，不可读不可写，只用于获取信息
        if let Some(file) = open_file(path.as_str(), file, OpenFlags::empty()) {
            if file.get_stat(kstat) {
                return Ok(0);
            }
        } else if let Some(file) = open_file(path.as_str(), file, OpenFlags::DIR) {
            if file.get_stat(kstat) {
                return Ok(0);
            }
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 获取文件系统的信息
pub fn sys_statfs(path: *const u8, stat: *mut FsStat) -> SysResult {
    let file_path = unsafe { raw_ptr_to_ref_str(path) };
    if file_path == "/" {
        // 目前只支持访问根目录文件系统的信息
        origin_fs_stat(stat);
        Ok(0)
    } else {
        Err(ErrorNo::EINVAL)
    }
}
/// 从一个表示目录的文件描述符中获取目录名。
/// 如果这个文件描述符不是代表目录，则返回None
///
/// 内部会拿 inner 和 fd_manager 的锁，所以传入参数时不要先拿 task 里的锁
fn get_dir_from_fd(task: &Arc<TaskControlBlock>, dir_fd: i32) -> Option<String> {
    if dir_fd == AT_FDCWD {
        // 如果要求在当前路径下打开
        Some(String::from(task.inner.lock().dir.as_str()))
    } else {
        // 否则需要去找 dir_fd 获取路径
        if let Ok(Some(dir)) = task
            .fd_manager
            .lock()
            .get_file(dir_fd as usize)
            .map(|f| f.get_dir().map(|s| String::from(s)))
        {
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
fn resolve_path_from_fd<'a>(
    task: &Arc<TaskControlBlock>,
    dir_fd: i32,
    path: *const u8,
) -> Option<(String, &'a str)> {
    let file_path = unsafe { raw_ptr_to_ref_str(path) };
    //println!("file_path {}", file_path);
    if file_path.starts_with("/") {
        // 绝对路径
        Some((String::from("./"), &file_path[1..])) // 需要加上 '.'，因为 os 中约定根目录是以 '.' 开头
    } else {
        // 相对路径
        if let Some(dir) = get_dir_from_fd(task, dir_fd) {
            Some((dir, file_path))
        } else {
            return None;
        }
    }
}

/// 创建硬链接。成功时返回0，失败时返回-1
pub fn sys_linkat(
    old_dir_fd: i32,
    old_path: *const u8,
    new_dir_fd: i32,
    new_path: *const u8,
    _flags: u32,
) -> SysResult {
    let task = get_current_task().unwrap();
    if let Some((old_path, old_file)) = resolve_path_from_fd(&task, old_dir_fd, old_path) {
        if let Some((new_path, new_file)) = resolve_path_from_fd(&task, new_dir_fd, new_path) {
            if try_add_link(old_path, old_file, new_path, new_file) {
                return Ok(0);
            }
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 删除硬链接，并在链接数为0时实际删除文件。成功时返回0，失败时返回-1
pub fn sys_unlinkat(dir_fd: i32, path: *const u8, _flags: u32) -> SysResult {
    let task = get_current_task().unwrap();
    if let Some((path, file)) = resolve_path_from_fd(&task, dir_fd, path) {
        if try_remove_link(path, file) {
            return Ok(0);
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 挂载文件系统。成功时返回0，失败时返回-1。
///
/// 目前只是语义上实现，还没有真实板子上测试过
pub fn sys_mount(
    device: *const u8,
    mount_path: *const u8,
    fs_type: *const u8,
    _flags: u32,
    _data: *const u8,
) -> SysResult {
    let fs_type = unsafe { raw_ptr_to_ref_str(fs_type) };
    if fs_type != "vfat" {
        // 不支持挂载其他类型
        return Err(ErrorNo::EINVAL);
    }
    let task = get_current_task().unwrap();
    // 这里把 fd 写成"当前目录"，但其实如果内部发现路径是 '/' 开头，会用绝对路径替代。
    // 这也是其他类似调用 open/close/mkdir/linkat 等的逻辑
    if let Some((device_path, device_file)) = resolve_path_from_fd(&task, AT_FDCWD, device) {
        if let Some((mut mount_path, mount_file)) =
            resolve_path_from_fd(&task, AT_FDCWD, mount_path)
        {
            mount_path += mount_file;
            if !mount_path.ends_with('/') {
                // 挂载到的是一个目录，但用户输入目录时不一定加了 '/'
                mount_path.push('/');
            }
            if mount_fat_fs(device_path, device_file, mount_path) {
                return Ok(0);
            }
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 卸载文件系统。成功时返回0，失败时(目录不存在/未挂载等)返回-1。
///
/// 目前只是语义上实现，还没有真实板子上测试过
pub fn sys_umount(mount_path: *const u8, _flags: u32) -> SysResult {
    let task = get_current_task().unwrap();
    if let Some((mut mount_path, mount_file)) = resolve_path_from_fd(&task, AT_FDCWD, mount_path) {
        mount_path += mount_file;
        if !mount_path.ends_with('/') {
            mount_path.push('/');
        }
        if umount_fat_fs(mount_path) {
            return Ok(0);
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 创建目录，成功时返回 0，失败时返回 -1
///
/// - 如果path是相对路径，则它是相对于dirfd目录而言的。
/// - 如果path是绝对路径，则dirfd被忽略。
pub fn sys_mkdir(dir_fd: i32, path: *const u8, _user_mode: u32) -> SysResult {
    let task = get_current_task().unwrap();
    if let Some((parent_dir, file_path)) = resolve_path_from_fd(&task, dir_fd, path) {
        if mkdir(parent_dir.as_str(), file_path) {
            return Ok(0);
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 切换当前工作路径，如果以.开头，默认是相对路径；如果以/开头，默认是绝对路径。切换成功时返回0，失败时返回-1
///
/// 会先检查要切换到的路径是否存在。
pub fn sys_chdir(path: *const u8) -> SysResult {
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    let file_path = unsafe { raw_ptr_to_ref_str(path) };

    let new_path = {
        if file_path.starts_with("/") {
            String::from(".") + file_path
        } else {
            let current_path = &mut tcb_inner.dir;
            if !current_path.ends_with("/") {
                // 添加路径尾的斜杠
                *current_path += "/";
            }
            current_path.clone() + file_path
        }
    };
    //info!("new path = {}", new_path);
    if check_dir_exists(new_path.as_str()) {
        *(&mut tcb_inner.dir) = new_path;
        Ok(0)
    } else {
        Err(ErrorNo::EINVAL)
    }
}

/// 打开文件，返回对应的 fd。如打开失败，则返回 -1
pub fn sys_open(dir_fd: i32, path: *const u8, flags: u32, _user_mode: u32) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_fd_manager = task.fd_manager.lock();
    // 如果 fd 已满，则不再添加
    if task_fd_manager.is_full() {
        return Err(ErrorNo::EMFILE);
    }
    if let Some((parent_dir, file_path)) = resolve_path_from_fd(&task, dir_fd, path) {
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
        info!(
            "try open parent_dir={} file_path={} flag={:x}",
            parent_dir, file_path, flags
        );
        if let Some(open_flags) = OpenFlags::from_bits(flags) {
            info!("[{:#?}]", open_flags);
            //println!("opened");
            if let Some(node) = open_file(parent_dir.as_str(), file_path.as_str(), open_flags) {
                //println!("opened");
                if let Ok(fd) = task_fd_manager.push(node) {
                    info!("return fd {}", fd);
                    return Ok(fd);
                } else if open_flags.contains(OpenFlags::EXCL) {
                    // 要求创建文件却打开失败，说明是文件已存在
                    return Err(ErrorNo::EEXIST);
                }
            }
        }
    }
    Err(ErrorNo::ENOENT)
}

/// 关闭文件，成功时返回 0，失败时返回 -1
pub fn sys_close(fd: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_fd_manager = task.fd_manager.lock();
    if let Ok(_file) = task_fd_manager.remove_file(fd) {
        // 其实可以对 file 做最后处理。
        // 但此处不知道 file 的具体类型，所以还是推荐实现 Trait File 的类型自己写 Drop 时处理
        info!("close fd {fd}");
        Ok(0)
    } else {
        Err(ErrorNo::EINVAL)
    }
}

/// 创建管道，在 *pipe 记录读管道的 fd，在 *(pipe+1) 记录写管道的 fd。
/// 成功时返回 0，失败则返回 -1
///
/// 注意，因为
pub fn sys_pipe(pipe: *mut u32) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_fd_manager = task.fd_manager.lock();
    let (pipe_read, pipe_write) = Pipe::new_pipe();
    if let Ok(fd1) = task_fd_manager.push(Arc::new(pipe_read)) {
        if let Ok(fd2) = task_fd_manager.push(Arc::new(pipe_write)) {
            unsafe {
                *pipe = fd1 as u32;
                *pipe.add(1) = fd2 as u32;
            }
            info!("pipe: read {fd1}, write {fd2}");
            return Ok(0);
        } else {
            // 只成功插入了一个 fd。这种情况下要把 pipe_read 退出来
            let _ = task_fd_manager.remove_file(fd1);
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 复制一个 fd 中的文件到一个新 fd 中，成功时返回新的文件描述符，失败则返回 -1
pub fn sys_dup(fd: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_fd_manager = task.fd_manager.lock();
    if let Ok(new_fd) = task_fd_manager.copy_fd_anywhere(fd) {
        info!("dup: from {fd} to {new_fd}");
        Ok(new_fd)
    } else {
        Err(ErrorNo::EMFILE)
    }
}

/// 复制一个 fd 中的文件到指定的新 fd 中，成功时返回新的文件描述符，失败则返回 -1
pub fn sys_dup3(old_fd: usize, new_fd: usize) -> SysResult {
    info!("dup3: from {old_fd} to {new_fd}");
    let task = get_current_task().unwrap();
    if task.fd_manager.lock().copy_fd_to(old_fd, new_fd) {
        Ok(new_fd)
    } else {
        Err(ErrorNo::EINVAL)
    }
}

/// 获取目录项信息
pub fn sys_getdents64(fd: usize, buf: *mut Dirent64, len: usize) -> SysResult {
    let task = get_current_task().unwrap();
    if let Some(dir) = get_dir_from_fd(&task, fd as i32) {
        let entry_id = unsafe { (*buf).d_off as usize / DIR_ENTRY_SIZE };
        if let Some((is_dir, file_name)) = get_kth_dir_entry_info_of_path(dir.as_str(), entry_id) {
            unsafe {
                (*buf).d_ino = 1;
                (*buf).d_off += DIR_ENTRY_SIZE as i64;
                (*buf).d_reclen = DIR_ENTRY_SIZE as u16;
                (*buf).set_type(if is_dir {
                    Dirent64Type::DIR
                } else {
                    Dirent64Type::REG
                });
                let name_start = (buf as usize + (*buf).d_name_offset()) as *mut u8;
                // 算出还能放 d_name 的位置。其中字符串结尾加一个0保证最后不溢出
                let len = len - (*buf).d_name_offset() - 1;
                let copy_len = if len < file_name.len() {
                    len
                } else {
                    file_name.len()
                };
                let slice = core::slice::from_raw_parts_mut(name_start, copy_len);
                slice.copy_from_slice(&file_name.as_bytes());
                // 字符串结尾
                *name_start.add(copy_len) = 0;
                return Ok(copy_len + (*buf).d_name_offset());
            }
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 修改文件的访问时间和/或修改时间。
///
/// 如果 fir_fd < 0，它和 path 共同决定要找的文件；
/// 如果 fir_fd >=0，它就是文件对应的 fd
/// 因为它要求文件访问的部分更多，因此放在 fs.rs 而非 times.rs
pub fn sys_utimensat(
    dir_fd: i32,
    path: *const u8,
    time_spec: *const TimeSpec,
    flags: UtimensatFlags,
) -> SysResult {
    info!(
        "dir_fd {}, path {:x}, ts {:x}, flags {:x}",
        dir_fd, path as usize, time_spec as usize, flags
    );
    let task = get_current_task().unwrap();
    if dir_fd != AT_FDCWD && dir_fd < 0 {
        return Err(ErrorNo::EBADF); // 错误的文件描述符
    }
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    //info!("fd {} buf {} len {}", fd, buf as usize, len);
    if dir_fd == AT_FDCWD && task_vm.manually_alloc_page(path as usize).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }

    // 获取需要设置的新时间
    let (new_atime, new_mtime) = if time_spec as usize == 0 {
        (TimeSpec::get_current(), TimeSpec::get_current())
    } else {
        if task_vm.manually_alloc_page(time_spec as usize).is_err() {
            return Err(ErrorNo::EFAULT); // 地址不合法
        }
        unsafe { (*time_spec, *time_spec.add(1)) }
    };
    if dir_fd > 0 {
        if let Ok(file) = fd_manager.get_file(dir_fd as usize) {
            file.set_time(&new_atime, &new_mtime);
            return Ok(0);
        }
    } else if let Some((parent_dir, file_path)) = resolve_path_from_fd(&task, dir_fd, path) {
        if check_file_exists(parent_dir.as_str(), file_path) {
            if let Some(file) = open_file(parent_dir.as_str(), file_path, OpenFlags::empty()) {
                if file.set_time(&new_atime, &new_mtime) {
                    return Ok(0);
                }
            }
            return Err(ErrorNo::EINVAL);
        } else {
            let full_dir = if let Some(pos) = file_path.rfind('/') {
                parent_dir + &file_path[..pos]
            } else {
                parent_dir
            };
            if !check_dir_exists(full_dir.as_str()) {
                // 如果连目录都不存在
                return Err(ErrorNo::ENOTDIR);
            }
        }
    }
    Err(ErrorNo::ENOENT)
}

/// 修改文件指针位置
pub fn sys_lseek(fd: usize, offset: isize, whence: isize) -> SysResult {
    info!("lseek fd {} offset {} whence {}", fd, offset, whence);
    let task = get_current_task().unwrap();
    //let mut tcb_inner = task.inner.lock();
    if let Ok(file) = task.fd_manager.lock().get_file(fd) {
        if let Some(new_offset) = file.seek(match whence {
            SEEK_SET => SeekFrom::Start(offset as u64),
            SEEK_CUR => SeekFrom::Current(offset as i64),
            SEEK_END => SeekFrom::End(offset as i64),
            _ => {
                return Err(ErrorNo::EINVAL);
            }
        }) {
            return Ok(new_offset);
        } else {
            return Err(ErrorNo::EINVAL);
        }
    }
    Err(ErrorNo::EBADF)
}

/// 设置文件属性。目前支持的比较少
pub fn sys_fcntl64(fd: usize, cmd: usize, arg: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let mut fd_manager = task.fd_manager.lock();
    info!("fcntl {fd} {cmd}");
    if let Ok(file) = fd_manager.get_file(fd) {
        return match Fcntl64Cmd::try_from(cmd) {
            Ok(Fcntl64Cmd::F_DUPFD) => {
                // 复制 fd
                if let Ok(new_fd) = fd_manager.copy_fd_anywhere(fd) {
                    Ok(new_fd)
                } else {
                    Err(ErrorNo::EMFILE)
                }
            },
            Ok(Fcntl64Cmd::F_GETFD) => {
                if file.get_status().contains(OpenFlags::CLOEXEC) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            },
            Ok(Fcntl64Cmd::F_SETFD) => {
                if file.set_close_on_exec((arg & 1) != 0) {
                    Ok(0)
                } else {
                    Err(ErrorNo::EINVAL)
                }
            },
            Ok(Fcntl64Cmd::F_GETFL) => Ok(file.get_status().bits() as usize),
            Ok(Fcntl64Cmd::F_SETFL) => {
                if let Some(flags) = OpenFlags::from_bits(arg as u32) {
                    if file.set_status(flags) {
                        return Ok(0);
                    }
                }
                Err(ErrorNo::EINVAL)
            },
            Ok(Fcntl64Cmd::F_DUPFD_CLOEXEC) => {
                if let Ok(new_fd) = fd_manager.copy_fd_anywhere(fd) {
                    if file.set_close_on_exec((arg & 1) != 0) {
                        Ok(new_fd)
                    } else {
                        // 如果不能设置 CLOEXEC 位，则删除文件并返回错误
                        fd_manager.remove_file(new_fd).unwrap();
                        Err(ErrorNo::EMFILE)
                    }
                } else {
                    Err(ErrorNo::EMFILE)
                }
            },
            _ => Err(ErrorNo::EINVAL),
        };
    }
    Err(ErrorNo::EBADF)
}

/// 从 in_fd 读取最多 count 个字符，存到 out_fd 中。
/// - 如果 offset != 0，则其指定了 in_fd 中文件的偏移，此时完成后会修改 offset 为读取后的位置，但不更新文件内部的 offset
/// - 否则，正常更新文件内部的 offset
pub fn sys_sendfile64(out_fd: usize, in_fd: usize, offset: *mut usize, count: usize) -> SysResult {
    //file.seek(SeekFrom::Current(0)).unwrap()
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    info!("sendfile out fd {out_fd} in fd {in_fd} offset {:x} count {count}", offset as usize);
    if let Ok(out_file) = fd_manager.get_file(out_fd) {
        if let Ok(in_file) = fd_manager.get_file(in_fd) {
            let current_pos = if offset as usize == 0 {
                in_file.seek(SeekFrom::Current(0)).unwrap_or(0)
            } else {
                if task_vm.manually_alloc_page(offset as usize).is_err() {
                    return Err(ErrorNo::EFAULT); // 地址不合法
                }
                if let Some(pos) = in_file.seek(SeekFrom::Start(unsafe {*offset} as u64)) {
                    pos
                } else { // 如果指定的 offset 无法取到，则直接返回
                    return Err(ErrorNo::ESPIPE);
                }
            };
            // 读取最多 count 字符
            // todo: 使用 buffer 避免一次读取太多到内存里
            let mut buf = vec![0u8; count.min(SENDFILE_BUFFER_SIZE)];
            
            if let Some(read_len) = in_file.read(&mut buf) {
                if let Some(write_len) = out_file.write(&buf[..read_len]) {
                    if offset as usize != 0 { // offset 非零则要求不更新实际文件，更新这个用户给的值
                        unsafe {
                            *offset += write_len;
                        }
                        in_file.seek(SeekFrom::Start(current_pos as u64)).unwrap_or(0);
                    } else if write_len != read_len { // 否则更新实际文件，此时如果写不完要退回去
                        in_file.seek(SeekFrom::Current(write_len as i64 - read_len as i64)).unwrap_or(0);
                    }
                    return Ok(write_len);
                }
            }
            return Err(ErrorNo::EINVAL);
        }
    }
    Err(ErrorNo::EBADF)
}