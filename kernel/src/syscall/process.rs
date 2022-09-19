//! 与进程相关的系统调用

use super::{
    resolve_clone_flags_and_signal, ErrorNo, MMAPFlags, RLimit, SysResult, UtsName, WaitFlags,
    MMAPPROT, MSyncFlags, RLIMIT_AS, RLIMIT_NOFILE, RLIMIT_STACK, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK,
};
use crate::{
    constants::{SIGSET_SIZE_IN_BYTE, USER_STACK_SIZE, USER_VIRT_ADDR_LIMIT, USE_MSYNC},
    file::{SeekFrom, BackEndFile},
    signal::{send_signal, Bitset, SigAction, SignalNo},
    memory::{page_offset, align_up, align_down},
    task::{
        exec_new_task, exit_current_task, get_current_task, push_task_to_scheduler, signal_return,
        suspend_current_task,
    },
    utils::{raw_ptr_to_string, str_ptr_array_to_vec_string},
};
use core::mem::size_of;

/// 进程退出，并提供 exit_code 供 wait 等 syscall 拿取
pub fn sys_exit(exit_code: i32) -> ! {
    //println!("[kernel] Application exited with code {}", exit_code);
    exit_current_task(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// 进程主动放弃时间片，立即切换到其他进程执行
pub fn sys_yield() -> SysResult {
    suspend_current_task();
    Ok(0)
}

/// 获取当前进程的 pid。
/// 如果该核没有正在运行的线程，则直接 panic
pub fn sys_getpid() -> SysResult {
    Ok(get_current_task().unwrap().get_pid_num())
}

/// 获取父进程的 pid。
/// 如果该核没有正在运行的线程，则直接 panic
///
/// 一个进程的父进程在它被 clone(fork) 时就确定了，但退出时它的状态信息会被移交给初始进程。
/// 当然如果一个用户进程已经退出，就不可能再调用 syscall 获取 ppid 了
pub fn sys_getppid() -> SysResult {
    Ok(get_current_task().unwrap().get_ppid())
}

/// 获取当前线程的编号。
/// 每个进程的初始线程的编号就是它的 pid
pub fn sys_gettid() -> SysResult {
    Ok(get_current_task().unwrap().get_tid_num())
}

/// 修改用户堆大小，
///
/// - 如输入 brk 为 0 ，则返回堆顶地址
/// - 否则，尝试修改堆顶为 brk，成功时返回0，失败时返回-1。
pub fn sys_brk(brk: usize) -> SysResult {
    if brk == 0 {
        Ok(get_current_task().unwrap().get_user_heap_top())
    } else {
        //info!("user try to move brk at {:x}", brk);
        Ok(get_current_task().unwrap().set_user_heap_top(brk)) // 如果设置不合法，会保持不变并返回原来的堆顶
    }
    //Err(ErrorNo::ENOMEM)
}

/// 创建一个子任务，如成功，返回其 tid
pub fn sys_clone(
    flags: usize,
    user_stack: usize,
    ptid: usize,
    tls: usize,
    ctid: usize,
) -> SysResult {
    let (clone_flags, signal) = resolve_clone_flags_and_signal(flags);
    info!(
        "clone: flags {:#?} signal {} ptid {:x} tls {:x} ctid {:x}",
        clone_flags, signal as usize, ptid, tls, ctid
    );
    let user_stack = if user_stack == 0 {
        None
    } else {
        Some(user_stack)
    };
    let old_task = get_current_task().unwrap();
    // 生成新任务。注意 from_clone 方法内部已经把对用户的返回值设成了0
    // 第二个参数指定了子任务退出时是否发送 SIGCHLD
    let new_task = old_task.from_clone(
        user_stack,
        signal == SignalNo::SIGCHLD,
        clone_flags,
        tls,
        ptid,
        ctid,
    );
    // 获取新进程的 pid。必须提前在此拿到 usize 形式的 pid，因为后续 new_task 插入任务队列后就不能调用它的方法了
    let new_task_tid = new_task.get_tid_num();
    // 将新任务加入调度器
    push_task_to_scheduler(new_task);
    //println!("new task {new_task_tid}");
    //println!("create time {}", crate::timer::get_time());
    Ok(new_task_tid)
    /*
    if signal == SignalNo::SIGCHLD { // 子进程
        let user_stack = if user_stack == 0 { None } else { Some(user_stack) };
        sys_fork(user_stack)
    } else {
        info!("flags {:#?} user_stack {:x}, ptid {:x} tls {:x} ctid {:x}", clone_flags, user_stack, ptid, tls, ctid);
        return -1
    }
    */
}

/// 复制当前进程
///
/// 如 user_stack 为 None，则沿用原进程的用户栈地址。
///
/// 目前 fork 的功能由 sys_clone 接管，所以不再是 pub 的
/*
fn sys_fork(user_stack: Option<usize>) -> SysResult {
    let old_task = get_current_task().unwrap();
    // 生成新进程。注意 from_fork 方法内部已经把对用户的返回值设成了0
    let new_task = old_task.from_clone(user_stack);
    // 获取新进程的 pid。必须提前在此拿到 usize 形式的 pid，因为后续 new_task 插入任务队列后就不能调用它的方法了
    let new_task_pid = new_task.get_pid_num();
    // 将新任务加入调度器
    push_task_to_scheduler(new_task);
    unsafe {
        let trap_context =  old_task.kernel_stack.get_first_context();
        println!("parent sepc {:x} stack {:x} new_task_pid {}", (*trap_context).sepc, (*trap_context).get_sp(), new_task_pid);
    };
    new_task_pid as isize
}
*/
/// 将当前进程替换为指定用户程序。
///
/// 环境变量留了接口但目前未实现
pub fn sys_execve(path: *const u8, args: *const usize, mut _envs: *const usize) -> SysResult {
    sys_exec(path, args)
}

/// 将当前进程替换为指定用户程序。
///
/// 如果找到这个名字的用户程序，返回 argc(参数个数)；
/// 如果没有找到这个名字的用户程序，则返回 -1
fn sys_exec(path: *const u8, args: *const usize) -> SysResult {
    // 因为这里直接用用户空间提供的虚拟地址来访问，所以一定能连续访问到字符串，不需要考虑物理地址是否连续。
    // 把路径和参数复制到内核里。因为上面的 slice 在用户空间中，在 exec 中会被 drop 掉。
    let app_name = unsafe { raw_ptr_to_string(path) };
    let args = unsafe { str_ptr_array_to_vec_string(args) };
    // 而且目前认为所有用户程序在根目录下，所以直接把路径当作文件名
    if get_current_task().unwrap().exec(&app_name, args) {
        exec_new_task();
        Ok(0)
    } else {
        sys_exit(0);
        //Err(ErrorNo::EINVAL)
    }
}

/// 等待子进程执行完成。如果它还没完成，则先切换掉
///
/// 目前只支持 WNOHANG 选项
pub fn sys_wait4(pid: isize, exit_code_ptr: *mut i32, option: WaitFlags) -> SysResult {
    info!("sys_wait4 {}, {:x}, {:#?}",
          pid, exit_code_ptr as usize, option);
    loop {
        let child_pid = waitpid(pid, exit_code_ptr);
        // 找不到子进程，直接返回-1
        if child_pid == -1 {
            return Err(ErrorNo::EINVAL);
        } else if child_pid == -2 {
            if option.contains(WaitFlags::WNOHANG) {
                return Ok(0);
            } else {
                info!("find child but suspend");
                suspend_current_task();
            }
        } else {
            info!("find child and return {}", child_pid);
            return Ok(child_pid as usize);
        }
    }
}

/// 等待一个子进程执行完成
///
/// 1. 如果找不到对应 pid 的进程，或者它不是调用进程的子进程，返回 -1
/// 2. 如果能找到，但该子进程没有运行结束，返回 -2
/// 3. 否则，返回这个进程的 pid。
/// 3.1 如果 exit_code_ptr != 0，则将子进程的 exit_code 写入 exit_code_ptr
fn waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let request_pid = pid as usize;
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 找到这个子进程并返回它在 children 数组里的下标。
    // 如果找不到，它设为 -1; 如果找到了但没结束，它设为 -2
    let mut flag: isize = -1;
    let mut exit_code: i32 = -1;
    let mut pid_found: isize = pid;
    for (idx, child) in tcb_inner.children.iter().enumerate() {
        //info!("waitpid child iter {:?}", child.pid);
        // 等待任意的子进程
        if pid == -1 {
            flag = -2;
            if let Some(code) = child.get_code_if_exit() {
                exit_code = code;
                flag = idx as isize;
                pid_found = child.get_pid_num() as isize;
                break;
            }
        }
        // 找到这个子进程了
        else if child.get_pid_num() == request_pid {
            // 这里拿着当前进程的锁，要求获取子进程的锁
            // 其实内部用的是 try_lock：
            // 因为如果子进程已退出，则一定可以拿到锁;
            // 反之如果拿不到锁，说明子进程一定还在运行，也就不用去拿了
            if let Some(code) = child.get_code_if_exit() {
                exit_code = code;
                flag = idx as isize;
            } else {
                flag = -2;
            }
            break;
        }
    }
    //println!("wait flag {} exit_code {} pid_found {} write_to_ptr {:x}", flag, exit_code, pid_found, exit_code_ptr as usize);
    /*
    if task.get_pid_num() == 2 {
        println!("sons = {}, flag {} code {}", tcb_inner.children.len(), flag, exit_code);
        if flag >= 0 && exit_code < 0 {
            panic!("");
        }
    }
    */
    if flag >= 0 {
        let _child = tcb_inner.children.remove(flag as usize);
        if exit_code_ptr as usize != 0 {
            unsafe {
                //info!("write exit code {}", exit_code);
                *exit_code_ptr = exit_code << 8;
            }
        }
        pid_found
    } else {
        flag
    }
}

/// 映射一段内存
pub fn sys_mmap(
    start: usize,
    len: usize,
    prot: MMAPPROT,
    flags: MMAPFlags,
    fd: i32,
    offset: usize,
) -> SysResult {
    info!(
        "mmap start={:x} len={:x} prot=[{:#?}] flags=[{:#?}] fd={} offset={:x}",
        start, len, prot, flags, fd, offset
    );
    /*
    // 检查是否区间不是按页aligned的
    if page_offset(start) != 0 || page_offset(start + len) != 0 {
        return Err(ErrorNo::EINVAL);
    }*/
    // redis 似乎会调用非按页 aligned 的 mmap，也可能是之前给它的参数没有传对
    // 所以现在这两个参数由内核手动对齐到页，不报错
    let len = align_up(start + len) - align_down(start);
    let start = align_down(start);
    // start == 0 表明需要OS为其找一段内存，而 MAP_FIXED 表明必须 mmap 在固定位置。两者是冲突的
    if start == 0 && flags.contains(MMAPFlags::MAP_FIXED) {
        return Err(ErrorNo::EINVAL);
    }
    // 是否可以放在任意位置
    let anywhere = start == 0 || !flags.contains(MMAPFlags::MAP_FIXED);
    let task = get_current_task().unwrap();
    let tcb_inner = task.inner.lock();

    //不实际映射到文件
    if flags.contains(MMAPFlags::MAP_ANONYMOUS) {
        drop(tcb_inner);
        // 根据linux规范需要 fd 设为 -1 且 offset 设为 0
        if fd == -1 && offset == 0 {
            if let Some(start) = task.mmap(start, start + len, prot.into(), None, anywhere) {
                return Ok(start);
            }
        }
    } else if let Ok(file) = task.fd_manager.lock().get_file(fd as usize) {
        //确认可以seek才获取文件，否则后续 lazy alloc 时不好处理
        if let Some(_off) = file.seek(SeekFrom::Start(offset as u64)) {
            // file 在从 fd 中拿的时候已经是 clone 了，所以这里可以直接传给 backend
            let backend = BackEndFile::new(file, offset, prot.into());
            drop(tcb_inner);
            // mmap 内部需要拿 inner 锁
            if let Some(start) =
                task.mmap(start, start + len, prot.into(), Some(backend), anywhere)
            {
                return Ok(start);
            }
        }
    }
    Err(ErrorNo::EINVAL)
}

/// 取消映射一段内存
pub fn sys_munmap(start: usize, len: usize) -> SysResult {
    info!("start {:x}, len {}", start, len);
    // 从语义上说， munmap 是一定成功的，即使没有删除到任何区间
    if get_current_task().unwrap().munmap(start, start + len) {
        Ok(0)
    } else {
        Err(ErrorNo::EINVAL)
    }
}

/// 映射一段内存
pub fn sys_mprotect(start: usize, len: usize, prot: MMAPPROT) -> SysResult {
    info!(
        "try mprotect start={:x} len={:x} prot=[{:#?}]",
        start, len, prot
    );
    if get_current_task().unwrap().mprotect(start, start + len, prot.into()) {
        Ok(0)
    } else {
        Err(ErrorNo::EINVAL)
    }
}

/// 映射一段内存
pub fn sys_msync(
    start: usize,
    len: usize,
    flags: MSyncFlags,
) -> SysResult {
    if !USE_MSYNC {
        return Ok(0);
    }
    info!("try msync start={:x} len={:x} flags=[{:#?}]", start, len, flags);
    // 检查是否区间不是按页aligned的
    if page_offset(start) != 0 || page_offset(start + len) != 0 {
        return Err(ErrorNo::EINVAL);
    }
    if flags.contains(MSyncFlags::INVALIDATE) {
        warn!("MSyncFlags::INVALIDATE is unsupported!");
        return Ok(0);
    }
    if get_current_task().unwrap().msync(start, start + len) {
        Ok(0)
    } else {
        Err(ErrorNo::ENOMEM)
    }
}

/// 获取系统信息
pub fn sys_uname(uts: *mut UtsName) -> SysResult {
    unsafe {
        (*uts) = UtsName::default();
    }
    Ok(0)
}

/// 获取用户 id。在实现多用户权限前默认为最高权限
pub fn sys_getuid() -> SysResult {
    Ok(0)
}

/// 获取有效用户 id，即相当于哪个用户的权限。在实现多用户权限前默认为最高权限
pub fn sys_geteuid() -> SysResult {
    Ok(0)
}

/// 获取用户组 id。在实现多用户权限前默认为最高权限
pub fn sys_getgid() -> SysResult {
    Ok(0)
}

/// 获取有效用户组 id，即相当于哪个用户的权限。在实现多用户权限前默认为最高权限
pub fn sys_getegid() -> SysResult {
    Ok(0)
}

/// 向 pid 指定的进程发送信号。
/// 如果进程中有多个线程，则会发送给任意一个未阻塞的线程。
///
/// pid 有如下情况
/// 1. pid > 0，则发送给指定进程
/// 2. pid = 0，则发送给所有同组进程
/// 3. pid = -1，则发送给除了初始进程(pid=1)外的所有当前进程有权限的进程
/// 4. pid < -2，则发送给组内 pid 为参数相反数的进程
///
/// 目前 2/3/4 未实现。对于 1，仿照 zCore 的设置，认为**当前进程自己或其直接子进程** 是"有权限"或者"同组"的进程。
pub fn sys_kill(pid: isize, signal_id: isize) -> SysResult {
    info!("kill pid {}, signal id {}", pid, signal_id);
    if pid > 0 && signal_id > 0 {
        //println!("kill pid {}, signal id {}", pid, signal_id);
        send_signal(pid as usize, signal_id as usize);
        Ok(0)
    } else if pid == 0 {
        Err(ErrorNo::ESRCH)
    } else { // 如果 signal_id == 0，则仅为了检查是否存在对应进程，此时应该返回参数错误。是的，用户库是会刻意触发这个错误的
        Err(ErrorNo::EINVAL)
    }
}

/// 向 tid 指定的线程发送信号。
///
/// 在 `https://man7.org/linux/man-pages/man2/tkill.2.html` 中，建议使用 tgkill 替代，
/// 这需要多加一个 tgid 参数，以防止错误的线程( tid 已被删除后重用)发送信号。
/// 但 libc 的测例中仍会使用这个 tkill
pub fn sys_tkill(tid: isize, signal_id: isize) -> SysResult {
    //info!("tkill tid {}, signal id {}", tid, signal_id);
    if tid > 0 {
        send_signal(tid as usize, signal_id as usize);
        Ok(0)
    } else {
        Err(ErrorNo::EINVAL)
    }
}

/// 改变当前线程屏蔽的信号类型。
///
/// 所有信号类型存放在 sigsetsize Byte 大小的一个 bitset 里(因为是riscv64，默认为 8)
/// 根据 how 将目前的信号类型对 set 取并集/差集或直接设为 set，并将旧信号存入 old_set 中。
///
/// 如果 set 为 0，则不设置；如果 old_set 为 0，则不存入。
pub fn sys_sigprocmask(
    how: i32,
    set: *const usize,
    old_set: *mut usize,
    sigsetsize: usize,
) -> SysResult {
    if sigsetsize != SIGSET_SIZE_IN_BYTE {
        return Err(ErrorNo::EINVAL);
    }

    // 这里仅输出调试信息，与处理无关
    info!(
        "how {}, set {:x}",
        how,
        if set as usize == 0 {
            0
        } else {
            unsafe { *set }
        }
    );

    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let mut receiver = task.signal_receivers.lock();

    if old_set as usize != 0 {
        // old_set 非零说明要求写入到这个地址
        if task_vm.manually_alloc_page(old_set as usize).is_err() {
            return Err(ErrorNo::EINVAL); // 地址不合法
        }
        unsafe {
            *old_set = receiver.mask.0;
        }
    }
    if set as usize != 0 {
        // set 非零时才考虑 how 并修改
        if task_vm.manually_alloc_page(set as usize).is_err() {
            return Err(ErrorNo::EINVAL); // 地址不合法
        }
        let set_val = Bitset::new(unsafe { *set });
        match how {
            SIG_BLOCK => receiver.mask.get_union(set_val),
            SIG_UNBLOCK => receiver.mask.get_difference(set_val),
            SIG_SETMASK => receiver.mask.set_new(set_val),
            _ => {
                return Err(ErrorNo::EINVAL);
            }
        };
    }
    Ok(0)
}

/// 改变当前进程的信号处理函数。
///
/// 如果 action 为 0，则不设置；如果 old_action 为 0，则不存入。
pub fn sys_sigaction(
    signum: usize,
    action: *const SigAction,
    old_action: *mut SigAction,
) -> SysResult {
    if signum == SignalNo::SIGKILL as usize || signum == SignalNo::SIGSTOP as usize {
        return Err(ErrorNo::EINVAL); // 特殊信号不能被覆盖
    }
    let task = get_current_task().unwrap();
    let mut task_vm = task.vm.lock();
    let mut handler = task.signal_handlers.lock();

    unsafe {
        if action as usize != 0 {
            info!(
                "when receive signal {:#x?} action {:#x?}",
                SignalNo::from(signum),
                *action
            );
        }
    }

    let old_addr = old_action as usize;
    if old_addr != 0 {
        // old_set 非零说明要求写入到这个地址
        if task_vm.manually_alloc_page(old_addr).is_err()
            || task_vm
                .manually_alloc_page(old_addr + size_of::<SigAction>() - 1)
                .is_err()
        {
            return Err(ErrorNo::EINVAL); // 地址不合法
        }
        handler.get_action(signum, old_action);
    }

    let addr = action as usize;
    if addr != 0 {
        // set 非零时才考虑 how 并修改
        if task_vm.manually_alloc_page(addr).is_err()
            || task_vm
                .manually_alloc_page(addr + size_of::<SigAction>() - 1)
                .is_err()
        {
            return Err(ErrorNo::EINVAL); // 地址不合法
        }
        handler.set_action(signum, action);
    }
    Ok(0)
}

/// 从信号处理过程中返回，即恢复信号处理前的用户程序上下文。
///
/// sigreturn 没有返回值，因此也不该写入 a0。
/// 但为了 syscall 通用性考虑，这里的实现还是返回了原本上下文中 a0 的值，即相当于实际做了 `a0 = a0;` 的操作
///
/// 一般由 libc 库调用。
pub fn sys_sigreturn() -> SysResult {
    let a0 = signal_return();
    if a0 == -1 {
        Err(ErrorNo::EINVAL)
    } else {
        Ok(a0 as usize)
    }
}

/// 设置 clear_child_tid 属性并返回 tid。
/// 这个属性会使得线程退出时发送:
/// `futex(clear_child_tid, FUTEX_WAKE, 1, NULL, NULL, 0);`
pub fn sys_set_tid_address(addr: usize) -> SysResult {
    info!("set tid addresss to {:x}", addr);
    get_current_task().unwrap().set_tid_address(addr);
    sys_gettid()
}

/// 修改一些资源的限制
///
/// pid 设为0时，表示应用于自己
pub fn sys_prlimt64(
    pid: usize,
    resource: i32,
    new_limit: *const RLimit,
    old_limit: *mut RLimit,
) -> SysResult {
    info!("pid {} resource {}", pid, resource);
    if pid == 0 {
        let task = get_current_task().unwrap();
        let mut fd_manger = task.fd_manager.lock();

        match resource {
            RLIMIT_STACK => {
                if old_limit as usize != 0 {
                    unsafe {
                        *old_limit = RLimit {
                            rlim_cur: USER_STACK_SIZE as u64,
                            rlim_max: USER_STACK_SIZE as u64,
                        };
                    }
                }
            }
            RLIMIT_NOFILE => {
                if old_limit as usize != 0 {
                    let limit = fd_manger.get_limit();
                    unsafe {
                        *old_limit = RLimit {
                            rlim_cur: limit as u64,
                            rlim_max: limit as u64,
                        };
                    }
                }
                if new_limit as usize != 0 {
                    let new_limit = unsafe { (*new_limit).rlim_cur };
                    fd_manger.modify_limit(new_limit as usize);
                }
            }
            RLIMIT_AS => {
                if old_limit as usize != 0 {
                    unsafe {
                        *old_limit = RLimit {
                            rlim_cur: USER_VIRT_ADDR_LIMIT as u64,
                            rlim_max: USER_VIRT_ADDR_LIMIT as u64,
                        };
                    }
                }
            }
            _ => {}
        }
    }
    Ok(0)
}
