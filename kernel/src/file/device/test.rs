//! 运行比赛测试

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use lock::Mutex;
use core::slice::Iter;
use lazy_static::*;

pub use crate::task::Scheduler;
pub use crate::task::TaskControlBlock;
pub use crate::loaders::parse_user_app;
pub use crate::constants::{ROOT_DIR, NO_PARENT};

/*
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
*/

/// 加载下一个用户程序。
pub fn load_next_testcase() -> Option<Arc<TaskControlBlock>> {
    TESTCASES_ITER.lock().next().map_or_else(|| {
        TEST_STATUS.lock().final_info();
        None
    },|user_prog_name| {
        TEST_STATUS.lock().load(user_prog_name);
        Some(Arc::new(TaskControlBlock::from_app_name(ROOT_DIR, user_prog_name, NO_PARENT).unwrap()))
    })
}


/// 输出测试结果
pub fn show_testcase_result(exit_code: i32) {
    TEST_STATUS.lock().update_result(exit_code)
}

/// 运行测试时的状态机
struct TestStatus {
    cnt: usize,
    passed: usize,
    now: Option<&'static str>,
    failed_tests: Vec<&'static str>,
}

impl TestStatus {
    /// 初始化测试信息
    pub fn new(cases: &[&str]) -> Self {
        Self {
            cnt: cases.len(),
            passed: 0,
            now: None,
            failed_tests: Vec::new(),
        }
    }

    /// 输入测试
    pub fn load(&mut self, testcase: &&'static str) {
        info!(" --------------- load testcase: {} --------------- ", testcase);
        self.now = Some(&testcase);
    }

    /// 更新执行结果
    pub fn update_result(&mut self, exit_code: i32) {
        // 检查是否已有退出
        // 这一步检查是因为测例中可能使用 fork/clone，但每个进程退出都会执行 update_result
        // 这件事的本质原因是进程退出后 cpu_local 无法知道这是否是一个测例，如果需要改这一点就需要把 exit_code 留存 scheduler 为空。
        // 但如果这样改会导致需要更多的全局 lazy_static，在内核中不太合适
        if self.now == None {
            return;
        }
        //cnt += 1;
        match exit_code {
            0 => {
                info!(" --------------- test passed --------------- "); 
                self.passed += 1;
            },
            _ => {
                info!(" --------------- TEST FAILED, exit code = {} --------------- ", exit_code);
                self.failed_tests.push(self.now.unwrap());
            },
        }
        self.now = None;
    }

    /// 最终输出测试信息
    pub fn final_info(&self) {
        info!(" --------------- all test ended, passed {} / {} --------------- ", self.passed, self.cnt);
        info!(" --------------- failed tests: --------------- ");
        for &test in &self.failed_tests {
            info!("{}", test);
        }
        info!(" --------------- end --------------- ");
    }
}

lazy_static! {
    static ref TESTCASES_ITER: Mutex<Iter<'static, &'static str>> = Mutex::new(SAMPLE.into_iter());
    static ref TEST_STATUS: Mutex<TestStatus> = Mutex::new(TestStatus::new(SAMPLE));
    //static ref TEST_COUNT: Mutex<usize> = Mutex::new(0);
    //static ref TEST_PASSED: Mutex<usize> = Mutex::new(0);
}

pub const SAMPLE: &[&str] = &[
    "daemon_failure",
];

pub const TESTCASES: &[&str] = &[
    "argv",
    "basename",
    "clocale_mbfuncs",
    "clock_gettime",
    "crypt",
    "daemon_failure",
    "dirname",
    "dn_expand_empty",
    "dn_expand_ptr_0",
    "env",
    "fdopen",
    "fflush_exit",
    "fgets_eof",
    "fgetwc_buffering",
    "flockfile_list",
    "fnmatch",
    "fpclassify_invalid_ld80",
    "fscanf",
    "ftello_unflushed_append",
    "fwscanf",
    "getpwnam_r_crash",
    "getpwnam_r_errno",
    "iconv_open",
    "iconv_roundtrips",
    "inet_ntop_v4mapped",
    "inet_pton",
    "inet_pton_empty_last_field",
    "iswspace_null",
    "lrand48_signextend",
    "lseek_large",
    "malloc_0",
    "mbc",
    "mbsrtowcs_overflow",
    "memmem_oob",
    "memmem_oob_read",
    "memstream",
    "mkdtemp_failure",
    "mkstemp_failure",
    "pleval",
    "printf_1e9_oob",
    "printf_fmt_g_round",
    "printf_fmt_g_zeros",
    "printf_fmt_n",
    "pthread_cancel",
    "pthread_cancel_points",
    "pthread_cancel_sem_wait",
    "pthread_cond",
    "pthread_cond_smasher",
    "pthread_condattr_setclock",
    "pthread_exit_cancel",
    "pthread_once_deadlock",
    "pthread_robust_detach",
    "pthread_rwlock_ebusy",
    "pthread_tsd",
    "putenv_doublefree",
    "qsort",
    "random",
    "regex_backref_0",
    "regex_bracket_icase",
    "regex_ere_backref",
    "regex_escaped_high_byte",
    "regex_negated_range",
    "regexec_nosub",
    "rewind_clear_error",
    "rlimit_open_files",
    "scanf_bytes_consumed",
    "scanf_match_literal_eof",
    "scanf_nullbyte_char",
    "search_hsearch",
    "search_insque",
    "search_lsearch",
    "search_tsearch",
    "setjmp",
    "setvbuf_unget",
    "sigprocmask_internal",
    "snprintf",
    "socket",
    "sscanf",
    "sscanf_eof",
    "sscanf_long",
    "stat",
    "statvfs",
    "strftime",
    "string",
    "string_memcpy",
    "string_memmem",
    "string_memset",
    "string_strchr",
    "string_strcspn",
    "string_strstr",
    "strptime",
    "strtod",
    "strtod_simple",
    "strtof",
    "strtol",
    "strtold",
    "strverscmp",
    "swprintf",
    "syscall_sign_extend",
    "tgmath",
    "time",
    "udiv",
    "ungetc",
    "uselocale_0",
    "utime",
    "wcsncpy_read_overflow",
    "wcsstr",
    "wcsstr_false_negative",
    "wcstol",
];

/// 初赛测例
pub const PRELIMINARY_TESTCASES: &[&str] = &[
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
    "getdents",
    "getpid",
    "getppid",
    "gettimeofday",
    "mkdir_",
    "mmap",
    "mount",
    "munmap",
    "open",
    "openat",
    "pipe",
    "read",
    "sleep",
    "times",
    "umount",
    "uname",
    "unlink",
    "wait",
    "waitpid",
    "write",
    "yield",
];