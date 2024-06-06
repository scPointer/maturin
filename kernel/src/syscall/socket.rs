//! 关于 socket 的 syscall

use super::SysResult;
use crate::file::socket::*;
use crate::task::suspend_current_task;
use crate::{file::Socket, task::get_current_task};
use alloc::sync::Arc;
use base_file::OpenFlags;
use core::mem::size_of;
use syscall::ErrorNo;

/// 创建一个 socket
pub fn sys_socket(domain: usize, s_type: usize, protocol: usize) -> SysResult {
    let domain = match Domain::try_from(domain) {
        Ok(domain) => domain,
        Err(_) => {
            warn!("Invalid socket domain: {domain}");
            return Err(ErrorNo::EAFNOSUPPORT);
        }
    };
    let socket_type = match SocketType::try_from(s_type & (SOCKET_TYPE_MASK as usize)) {
        Ok(t) => t,
        Err(_) => {
            warn!("Invalid socket type: {s_type}");
            return Err(ErrorNo::EINVAL);
        }
    };
    info!(
        "SOCKET domain: {:?}, s_type: {:?}, protocol: {:x}",
        domain, socket_type, protocol
    );
    let task = get_current_task().unwrap();
    let mut fd_manager = task.fd_manager.lock();
    if let Ok(fd) = fd_manager.push(Arc::new(Socket::new(domain, socket_type, protocol))) {
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
    dest_addr: *const u8,
    _addr_len: usize,
) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let fd_manager = task.fd_manager.lock();
    if task_vm.manually_alloc_user_str(buf, len).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }
    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
    if let Ok(file) = fd_manager.get_file(fd) {
        // 这里不考虑进程切换.                             dest_addr可能为0
        if let Some(write_len) = file.sendto(slice, flags, dest_addr as usize) {
            return Ok(write_len);
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
    _flags: i32,
    _src_addr: *mut u8,
    _src_len_pos: *mut u32,
) -> SysResult {
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_user_str(buf, len).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }
    drop(task_vm);
    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    loop {
        let fd_manager = task.fd_manager.lock();
        if let Ok(file) = fd_manager.get_file(fd) {
            /* if let Some(read_len) = file.recvfrom(slice, flags, src_addr,
            unsafe { src_len_pos.as_mut().unwrap() }) */
            if let Some(read_len) = file.recvfrom(slice, 0, 0, &mut 0) {
                return Ok(read_len);
            }
            let fl = file.get_status();
            if fl.contains(OpenFlags::NON_BLOCK) {
                info!("sys_recvfrom flags: {:?}", fl);
                return Err(ErrorNo::EAGAIN);
            }
        } else {
            return Err(ErrorNo::EBADF);
        }

        drop(fd_manager);
        suspend_current_task(); // yield
    }
}

/// 绑定socket fd到指定地址的IP和Port
pub fn sys_bind(fd: usize, addr: *const u8, addr_len: usize) -> SysResult {
    info!("sys_bind: fd: {} addr: {:p} len: {}", fd, addr, addr_len);
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_user_str(addr, addr_len).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }
    let fd_manager = task.fd_manager.lock();
    if let Ok(file) = fd_manager.get_file(fd) {
        let sock = file.as_any().downcast_ref::<Socket>().unwrap();
        if let Some(_p) = sock.set_endpoint(addr, false) {
            Ok(0)
        } else {
            Err(ErrorNo::EINVAL)
        }
    } else {
        return Err(ErrorNo::EBADF);
    }
}

/// 设置socket为监听模式
pub fn sys_listen(fd: usize, backlog: usize) -> SysResult {
    info!("sys_listen: fd: {} backlog: {}", fd, backlog);
    let task = get_current_task().unwrap();
    let fd_manager = task.fd_manager.lock();
    if let Ok(file) = fd_manager.get_file(fd) {
        let sock = file.as_any().downcast_ref::<Socket>().unwrap();
        sock.set_listening(true);
        Ok(0)
    } else {
        Err(ErrorNo::EBADF)
    }
}

/// socket连接给的远程地址. 如完成TCP的三次握手
pub fn sys_connect(fd: usize, addr: *const u8, addr_len: usize) -> SysResult {
    info!("sys_connect: fd: {} addr: {:p} len: {}", fd, addr, addr_len);
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_user_str(addr, addr_len).is_err() {
        return Err(ErrorNo::EFAULT); // 地址不合法
    }

    let fd_manager = task.fd_manager.lock();
    if let Ok(file) = fd_manager.get_file(fd) {
        let sock = file.as_any().downcast_ref::<Socket>().unwrap();
        let laddr = IpAddr {
            family: 2,
            port: 0,
            addr: 0,
        };
        //设置好本地endpoint
        if let Some(lport) = sock.set_endpoint(&laddr as *const _ as *const u8, false) {
            // TCP submit a SYN
            // 注意转换成Big endian
            let laddr = IpAddr {
                family: 2,
                port: u16::to_be(lport),
                addr: u32::to_be(0x7f000001),
            };
            let slice = unsafe {
                core::slice::from_raw_parts(
                    &laddr as *const IpAddr as *const u8,
                    size_of::<IpAddr>(),
                )
            };
            //把本地地址告诉给远端, 暂用端口port+100
            if let Some(write_len) = file.sendto(slice, 100, addr as usize) {
                //设置好远程endpoint
                let rport = sock.set_endpoint(addr, true).unwrap_or(0);
                info!(
                    "sys_connect sent IpAddr {} from {} to {} len: {}",
                    size_of::<IpAddr>(),
                    lport,
                    rport,
                    write_len
                );
                // 这里没loop, 直接返回成功
                return Ok(0);
            } else {
                return Err(ErrorNo::ECONNREFUSED);
            }
        } else {
            return Err(ErrorNo::EINVAL);
        }
    } else {
        Err(ErrorNo::EBADF)
    }
}

/// 监听着的SOCK_STREAM类型的socket, 接受连接, 原socket不受影响，创建一个新的socket返回
pub fn sys_accept4(fd: usize, addr: *mut u8, addr_len: *mut u32, flags: i32) -> SysResult {
    info!(
        "sys_accept: fd: {} addr: {:p} len: {:p}",
        fd, addr, addr_len
    );
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    if task_vm.manually_alloc_page(addr as usize).is_err()
        || task_vm.manually_alloc_page(addr_len as usize).is_err()
    {
        return Err(ErrorNo::EINVAL);
    }
    drop(task_vm);
    loop {
        let mut fd_manager = task.fd_manager.lock();
        if let Ok(file) = fd_manager.get_file(fd) {
            //if file.ready_to_read()
            let mut buffer = [0u8; 64];
            //获取新连接的远端地址
            if let Some(read_len) = file.recvfrom(&mut buffer, 100, 0, &mut 0) {
                info!("accept recv: {}", read_len);
                if read_len != size_of::<IpAddr>() {
                    warn!("accept unknown IpAddr");
                }
                unsafe {
                    let taddr = core::slice::from_raw_parts_mut(addr as *mut u8, read_len);
                    taddr.copy_from_slice(&buffer[..read_len]);
                    *addr_len = read_len as u32;

                    let recv_addr = addr as *const IpAddr;
                    info!(
                        "sys_accept got IpAddr {} family: {:?}, IP: {:x}, Port: {}",
                        *addr_len,
                        (*recv_addr).family,
                        u32::from_be((*recv_addr).addr),
                        u16::from_be((*recv_addr).port)
                    );
                }

                //设置好远程endpoint
                let sock = file.as_any().downcast_ref::<Socket>().unwrap();
                sock.set_endpoint(addr, true).unwrap_or(0);

                // New Socket
                if let Ok(new_fd) = fd_manager.push(Arc::new(sock.clonew())) {
                    return Ok(new_fd);
                } else {
                    return Err(ErrorNo::EMFILE);
                }
            } else if let Some(fl) = OpenFlags::from_bits((flags as u32) & !SOCKET_TYPE_MASK) {
                if fl.contains(OpenFlags::NON_BLOCK) {
                    info!("sys_accept flags: {:?}", fl);
                    return Err(ErrorNo::EAGAIN);
                }
            }
        } else {
            return Err(ErrorNo::EBADF);
        }
        drop(fd_manager);
        suspend_current_task(); // yield
    }
}
