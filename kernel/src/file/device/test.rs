//! 运行比赛测试

#![deny(missing_docs)]

use alloc::sync::Arc;
use lock::Mutex;
use core::slice::Iter;
use lazy_static::*;

pub use crate::task::Scheduler;
pub use crate::task::TaskControlBlock;
pub use crate::loaders::parse_user_app;
pub use crate::constants::{ROOT_DIR, NO_PARENT};

/// 加载用户程序。
/// 因为是调度器 GLOBAL_TASK_SCHEDULER 初始化时就加载，所以不能用 task::push_task_to_scheduler
pub fn load_testcases(scheduler: &mut Scheduler) {
    info!("read testcases");
    let iter = TESTCASES.into_iter();
    for user_prog in TESTCASES {
        info!("{}", user_prog);
        let tcb = TaskControlBlock::from_app_name(ROOT_DIR, user_prog, NO_PARENT).unwrap();
        scheduler.push(Arc::new(tcb));
    }
}

pub fn load_next_testcase() -> Option<Arc<TaskControlBlock>> {
    TESTCASES_ITER.lock().next().map(|user_prog_name| {
        Arc::new(TaskControlBlock::from_app_name(ROOT_DIR, user_prog_name, NO_PARENT).unwrap())
    })
}

lazy_static! {
    static ref TESTCASES_ITER: Mutex<Iter<'static, &'static str>> = Mutex::new(TESTCASES.into_iter());
}

pub const SAMPLE: &[&str] = &[
    "mmap",
    "munmap",
];

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
    "fstat",
    "getcwd",
    /*
    "getdents",
    */
    "getpid",
    "getppid",
    "gettimeofday",
    "mkdir_",
    "mount",
    "munmap",
    "open",
    "openat",
    "pipe",
    "read",
    "sleep",
    "times",
    "umount",
    /*
    "uname",
    */
    "unlink",
    "wait",
    "waitpid",
    "write",
    "yield",
];