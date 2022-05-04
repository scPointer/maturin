//! 与进程相关的系统调用


#![deny(missing_docs)]

use alloc::sync::Arc;

use crate::task::{
    exit_current_task, 
    suspend_current_task, 
    get_current_task,
    push_task_to_scheduler,
};
use crate::task::TaskStatus;
use crate::timer::get_time_ms;
use crate::utils::get_str_len;
use crate::loader::get_app_data_by_name;

/// 进程退出，并提供 exit_code 供 wait 等 syscall 拿取
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_task(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// 进程主动放弃时间片，立即切换到其他进程执行
pub fn sys_yield() -> isize {
    suspend_current_task();
    0
}

/// 获取系统时间(单位为毫秒)
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

/// 获取当前进程的 pid。
/// 如果该核没有正在运行的线程，则直接 panic
pub fn sys_getpid() -> isize {
    get_current_task().unwrap().get_pid_num() as isize
}

/// 复制当前进程
pub fn sys_fork() -> isize {
    let old_task = get_current_task().unwrap();
    // 生成新进程。注意 from_fork 方法内部已经把对用户的返回值设成了0
    let new_task = old_task.from_fork();
    // 获取新进程的 pid。必须提前在此拿到 usize 形式的 pid，因为后续 new_task 插入任务队列后就不能调用它的方法了
    let new_task_pid = new_task.get_pid_num();
    // 将新任务加入调度器
    push_task_to_scheduler(new_task);
    new_task_pid as isize
}

/// 将当前进程替换为指定用户程序。
/// 
/// 成功时返回 0 ; 如果没有找到这个名字的用户程序，则返回 -1
pub fn sys_exec(path: *const u8) -> isize {
    let len = unsafe { get_str_len(path) };
    // 因为这里直接用用户空间提供的虚拟地址来访问，所以一定能连续访问到字符串，不需要考虑物理地址是否连续
    let slice = unsafe { core::slice::from_raw_parts(path, len) };
    let string = core::str::from_utf8(slice).unwrap();
    if let Some(data) = get_app_data_by_name(string) {
        get_current_task().unwrap().exec(data);
        0
    } else {
        -1
    }
}

/// 等待一个子进程执行完成
/// 
/// 1. 如果找不到对应 pid 的进程，或者它不是调用进程的子进程，返回 -1
/// 2. 如果能找到，但该子进程没有运行结束，返回 -2
/// 3. 否则，返回这个进程的 pid，并将子进程的 exit_code 写入 exit_code_ptr
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let request_pid = pid as usize;
    let task = get_current_task().unwrap();
    let mut tcb_inner = task.inner.lock();
    // 找到这个子进程并返回它在 children 数组里的下标。
    // 如果找不到，它设为 -1; 如果找到了但没结束，它设为 -2
    let mut flag: isize = -1;
    let mut exit_code: i32 = -1;
    for (idx, child) in tcb_inner.children.iter().enumerate() {
        // 找到这个子进程了
        if child.get_pid_num() == request_pid {
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
    if flag >= 0 {
        let child = tcb_inner.children.remove(flag as usize);
        // 确认它没有其他引用了
        assert_eq!(Arc::strong_count(&child), 1);
        unsafe {*exit_code_ptr = exit_code; }
        pid
    } else {
        flag
    }
}