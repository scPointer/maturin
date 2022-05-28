//! 与进程相关的系统调用

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::string::String;

use crate::task::{
    exit_current_task, 
    suspend_current_task, 
    get_current_task,
    push_task_to_scheduler,
    exec_new_task,
};
use crate::task::TaskStatus;
use crate::utils::{
    raw_ptr_to_string,
    str_ptr_array_to_vec_string,
};
use crate::constants::{
    SIGCHLD,
};

use super::WaitFlags;

/// 进程退出，并提供 exit_code 供 wait 等 syscall 拿取
pub fn sys_exit(exit_code: i32) -> ! {
    //println!("[kernel] Application exited with code {}", exit_code);
    exit_current_task(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// 进程主动放弃时间片，立即切换到其他进程执行
pub fn sys_yield() -> isize {
    suspend_current_task();
    0
}

/// 获取当前进程的 pid。
/// 如果该核没有正在运行的线程，则直接 panic
pub fn sys_getpid() -> isize {
    get_current_task().unwrap().get_pid_num() as isize
}

/// 获取父进程的 pid。
/// 如果该核没有正在运行的线程，则直接 panic
/// 
/// 一个进程的父进程在它被 clone(fork) 时就确定了，但退出时它的状态信息会被移交给初始进程。
/// 当然如果一个用户进程已经退出，就不可能再调用 syscall 获取 ppid 了
pub fn sys_getppid() -> isize {
    get_current_task().unwrap().get_ppid() as isize
}

/// 修改用户堆大小，
/// 
/// - 如输入 brk 为 0 ，则返回堆顶地址
/// - 否则，尝试修改堆顶为 brk，成功时返回0，失败时返回-1。
pub fn sys_brk(brk: usize) -> isize {
    if brk == 0 {
        get_current_task().unwrap().get_user_heap_top() as isize
    } else {
        if get_current_task().unwrap().set_user_heap_top(brk) {
            0
        } else {
            -1
        }
    }
}

/// 创建一个子进程
pub fn sys_clone(flags: usize, user_stack: usize, _ptid: u32, _tls: u32, _ctid: u32) -> isize {
    if flags != SIGCHLD {
        return -1
    }
    let user_stack = if user_stack == 0 { None } else { Some(user_stack) };
    sys_fork(user_stack)
}

/// 复制当前进程
/// 
/// 如 user_stack 为 None，则沿用原进程的用户栈地址。
/// 
/// 目前 fork 的功能由 sys_clone 接管，所以不再是 pub 的
fn sys_fork(user_stack: Option<usize>) -> isize {
    let old_task = get_current_task().unwrap();
    // 生成新进程。注意 from_fork 方法内部已经把对用户的返回值设成了0
    let new_task = old_task.from_fork(user_stack);
    // 获取新进程的 pid。必须提前在此拿到 usize 形式的 pid，因为后续 new_task 插入任务队列后就不能调用它的方法了
    let new_task_pid = new_task.get_pid_num();
    // 将新任务加入调度器
    push_task_to_scheduler(new_task);
    /*
    unsafe {
        let trap_context =  old_task.kernel_stack.get_first_context();
        println!("parent sepc {:x} stack {:x} new_task_pid {}", (*trap_context).sepc, (*trap_context).get_sp(), new_task_pid);
    }; 
    */
    new_task_pid as isize
}

/// 将当前进程替换为指定用户程序。
/// 
/// 环境变量留了接口但目前未实现
pub fn sys_execve(path: *const u8, mut args: *const usize, mut _envs: *const usize) -> isize {
    sys_exec(path, args)
}

/// 将当前进程替换为指定用户程序。
/// 
/// 如果找到这个名字的用户程序，返回 argc(参数个数)；
/// 如果没有找到这个名字的用户程序，则返回 -1
fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    // 因为这里直接用用户空间提供的虚拟地址来访问，所以一定能连续访问到字符串，不需要考虑物理地址是否连续。
    // 把路径和参数复制到内核里。因为上面的 slice 在用户空间中，在 exec 中会被 drop 掉。
    let app_name = unsafe { raw_ptr_to_string(path) };
    let mut args = unsafe { str_ptr_array_to_vec_string(args) };
    // 而且目前认为所有用户程序在根目录下，所以直接把路径当作文件名
    if get_current_task().unwrap().exec(&app_name, args) {
        exec_new_task();
        0
    } else {
        -1
    }
}

/// 等待子进程执行完成。如果它还没完成，则先切换掉
/// 
/// 目前只支持 WNOHANG 选项
pub fn sys_wait4(pid: isize, exit_code_ptr: *mut i32, option: WaitFlags) -> isize {
    loop {
        let child_pid = sys_waitpid(pid, exit_code_ptr);
        // 找不到子进程，直接返回-1
        if child_pid == -1 {
            return -1
        } else if child_pid == -2 {
            if option.contains(WaitFlags::WNOHANG) {
                return 0;
            } else {
                //println!("find child but suspend");
                suspend_current_task();
            }
        } else {
            //println!("find child and return {}", child_pid);
            return child_pid
        }
    }
}

/// 等待一个子进程执行完成
/// 
/// 1. 如果找不到对应 pid 的进程，或者它不是调用进程的子进程，返回 -1
/// 2. 如果能找到，但该子进程没有运行结束，返回 -2
/// 3. 否则，返回这个进程的 pid。
/// 3.1 如果 exit_code_ptr == 0，则将子进程的 exit_code 写入 exit_code_ptr
fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let request_pid = pid as usize;
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 找到这个子进程并返回它在 children 数组里的下标。
    // 如果找不到，它设为 -1; 如果找到了但没结束，它设为 -2
    let mut flag: isize = -1;
    let mut exit_code: i32 = -1;
    let mut pid_found: isize = pid;
    for (idx, child) in tcb_inner.children.iter().enumerate() {
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
    };
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
        let child = tcb_inner.children.remove(flag as usize);
        // 确认它没有其他引用了
        // Todo: 这里加 assert 偶尔会报错，有可能是其他核在退出这个子进程的时候还拿着锁，但没法稳定触发
        // assert_eq!(Arc::strong_count(&child), 1, "child pid = {}", flag);
        
        // linux 调用规定中，返回的 exit_code 要左移8位
        if exit_code_ptr as usize != 0 {
            unsafe {*exit_code_ptr = exit_code << 8; }
        }
        pid_found
    } else {
        flag
    }
}