//! 关于 socket 的 syscall

use super::{SysResult, ErrorNo};
use crate::{file::Socket, task::get_current_task};
use alloc::sync::Arc;

/// 创建一个 socket
pub fn sys_socket(domain: usize, s_type: usize, protocol: usize) -> SysResult {
    let task = get_current_task().unwrap();
    let mut fd_manager = task.fd_manager.lock();
    if let Ok(fd) = fd_manager.push(Arc::new(Socket::new(domain, s_type, protocol))) {
        Ok(fd)
    } else {
        Err(ErrorNo::EMFILE)
    }
}

/// 发送消息，目的地在 dest_addr 的信息中
pub fn sys_sendto(
    fd: usize,
    buf: *const u8,
    len: usize,
    flags: i32,
    dest_addr: usize,
    addr_len: usize,
) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    if task_vm.manually_alloc_page(buf as usize).is_err()
        || task_vm.manually_alloc_page(buf as usize + len).is_err()
        || task_vm.manually_alloc_page(dest_addr).is_err()
        || task_vm.manually_alloc_page(dest_addr + addr_len).is_err()
    {
        return Err(ErrorNo::EINVAL);
    }
    let slice = unsafe { core::slice::from_raw_parts(buf, len) };

    if let Ok(file) = fd_manager.get_file(fd) {
        // 这里不考虑进程切换
        if let Some(write_len) = file.sendto(slice, flags, dest_addr) {
            return Ok(write_len)
        } else {
            return Err(ErrorNo::EINVAL);
        }
    } else {
        return Err(ErrorNo::EBADF);
    }
}

/// 收取消息，消息地址需要解析 dest_addr 获得
///
/// 消息的地址信息的长度(注意不是消息长度)将被存放在 src_len_pos 中
pub fn sys_recvfrom(
    fd: usize,
    buf: *mut u8,
    len: usize,
    flags: i32,
    src_addr: usize,
    src_len_pos: *mut u32,
) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    if task_vm.manually_alloc_page(buf as usize).is_err()
        || task_vm.manually_alloc_page(buf as usize + len).is_err()
        || task_vm.manually_alloc_page(src_addr).is_err()
        || task_vm.manually_alloc_page(src_len_pos as usize).is_err()
    {
        return Err(ErrorNo::EINVAL);
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    if let Ok(file) = fd_manager.get_file(fd) {
        // 这里不考虑进程切换
        if let Some(read_len) = file.recvfrom(slice, flags, src_addr, unsafe {
            src_len_pos.as_mut().unwrap()
        }) {
            return Ok(read_len);
        } else {
            return Err(ErrorNo::EINVAL);
        }
    } else {
        return Err(ErrorNo::EBADF);
    }
}
