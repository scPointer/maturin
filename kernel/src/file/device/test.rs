//! 运行比赛测试

#![deny(missing_docs)]

use alloc::sync::Arc;

pub use crate::task::Scheduler;
pub use crate::task::TaskControlBlock;
pub use crate::loaders::parse_user_app;
pub use crate::constants::ROOT_DIR;

/// 加载用户程序。
/// 因为是调度器 GLOBAL_TASK_SCHEDULER 初始化时就加载，所以不能用 task::push_task_to_scheduler
pub fn load_testcases(scheduler: &mut Scheduler) {
    println!("read testcases");
    for user_prog in TESTCASES {
        println!("{}", user_prog);
        let tcb = TaskControlBlock::from_app_name(ROOT_DIR, user_prog, 0).unwrap();
        scheduler.push(Arc::new(tcb));
    }
}

pub const TESTCASES: &[&str] = &[
    "brk",
    "chdir",
    "clone",
    "close",
    "dup",
    "dup2",
    "execve",
    "exit",
    "fork",
    /*
    "fstat",
    */
    "getcwd",
    /*
    "getdents",
    */
    "getpid",
    "getppid",
    "gettimeofday",
    "mkdir_",
    /*
    "mmap",
    "mount",
    "munmap",
    */
    "open",
    //"start",
    "openat",
    "pipe",
    "read",
    "sleep",
    "times",
    /*
    "umount",
    "uname",
    "unlink",
    */
    "wait",
    "waitpid",
    "write",
    "yield",
];