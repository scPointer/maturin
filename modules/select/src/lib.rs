//! 处理 pselect 相关的系统调用
//!
//! **该模块依赖 `task-trampoline`，因此使用该模块前，请先按照 `task-trampoline` 的文档说明进行初始化。**

#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use base_file::File;
use bitset::ShadowBitset;
use syscall::ErrorNo;
use task_trampoline::{get_file, manually_alloc_range, manually_alloc_type, suspend_current_task};

/// 获取 fd 指向文件的集合，
/// 每个文件存在 arc 里，每个 fd 值存在一个 usize 里，然后在用户地址原地清空建立一个 ShadowBitset。
///
/// 如果失败，如用户地址不合法 / fd 不存在，则返回对应错误
///
/// 这样做是因为，select / pselect 处理的 bitset 不长，也没有范围操作，但需要频繁读写，
/// 此时存在 vec 里反而比存在 bitset 里容易
fn init_fd_sets(
    addr: *mut usize,
    len: usize,
) -> Result<(Vec<Arc<dyn File>>, Vec<usize>, ShadowBitset), ErrorNo> {
    let shadow_bitset = unsafe { ShadowBitset::from_addr(addr, len) };
    if addr as usize == 0 {
        // 检查输入地址，如果为空则这个集合为空
        return Ok((Vec::new(), Vec::new(), shadow_bitset));
    }
    if manually_alloc_range(addr as usize, (addr as usize) + ((len + 63) & 63)).is_err() {
        // 其实还应检查 addr + ((len + 63) & 63)
        return Err(ErrorNo::EFAULT);
    }
    // 读取对应文件
    let mut fds: Vec<usize> = Vec::new();
    let mut files: Vec<Arc<dyn File>> = Vec::new();
    for fd in 0..len {
        if unsafe { shadow_bitset.check(fd) } {
            if let Some(file) = get_file(fd) {
                fds.push(fd);
                files.push(file);
            } else {
                return Err(ErrorNo::EBADF);
            }
        }
    }
    // 清空这一段的 bitset，直到之后 select 到可读/可写/异常的文件才写入
    unsafe {
        shadow_bitset.clear();
    }
    Ok((files, fds, shadow_bitset))
}

/// 实现 pselect 的系统调用
pub fn sys_pselect6(
    nfds: usize,
    readfds: *mut usize,
    writefds: *mut usize,
    exceptfds: *mut usize,
    timeout: *const timer::TimeSpec, // pselect 不会更新 timeout 的值，而 select 会
    _sigmask: *const usize,
) -> Result<usize, ErrorNo> {
    if nfds >= base_file::FD_LIMIT_HARD {
        return Err(ErrorNo::EINVAL);
    }
    let (rfile, rfd, rset) = init_fd_sets(readfds, nfds)?;
    let (wfile, wfd, wset) = init_fd_sets(writefds, nfds)?;
    let (efile, efd, eset) = init_fd_sets(exceptfds, nfds)?;
    // 过期时间
    // 注意 pselect 不会修改用户空间中的 timeout，所以需要内核自己记录
    // 这里用**时钟周期数**来记录，足够精确的同时 usize 也能存下。实际用微秒或者纳秒应该也没问题。
    let expire_time = if timeout as usize != 0 {
        if manually_alloc_type(timeout).is_err() {
            return Err(ErrorNo::EFAULT); // 无效地址
        }
        timer::get_time() + unsafe { (*timeout).get_ticks() }
    } else {
        usize::MAX // 没有过期时间
    };
    // 这里暂时不考虑 sigmask 的问题

    // info!(
    //     "pselect {nfds} {:#?} {:#?} {:#?} {}(now {})",
    //     rfd,
    //     wfd,
    //     efd,
    //     expire_time,
    //     timer::get_time()
    // );

    loop {
        // 已设置的 fd
        let mut set: usize = 0;
        if rset.is_valid() {
            // 如果设置了监视是否可读的 fd
            for i in 0..rfile.len() {
                if rfile[i].ready_to_read() {
                    unsafe {
                        rset.set(rfd[i]);
                    }
                    set += 1;
                }
            }
        }
        if wset.is_valid() {
            // 如果设置了监视是否可写的 fd
            for i in 0..wfile.len() {
                if wfile[i].ready_to_write() {
                    unsafe {
                        wset.set(wfd[i]);
                    }
                    set += 1;
                }
            }
        }
        if eset.is_valid() {
            // 如果设置了监视是否异常的 fd
            for i in 0..efile.len() {
                if efile[i].in_exceptional_conditions() {
                    unsafe {
                        eset.set(efd[i]);
                    }
                    set += 1;
                }
            }
        }
        if set > 0 {
            // 如果找到满足条件的 fd，则返回找到的 fd 数量
            return Ok(set);
        }
        // 否则暂时 block 住
        suspend_current_task();
        if timer::get_time() > expire_time {
            // 检查超时
            return Ok(0);
        }
    }
}
