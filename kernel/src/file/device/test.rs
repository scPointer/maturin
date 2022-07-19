//! 运行比赛测试

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
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
    },|&user_command| {
        let mut argv: Vec<String> = user_command.split(' ').map(|s| s.into()).collect();
        let argv = argv.drain_filter(|s| s != "").collect();
        TEST_STATUS.lock().load(&user_command.into());
        Some(Arc::new(TaskControlBlock::from_app_name(ROOT_DIR, NO_PARENT, argv).unwrap()))
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
    now: Option<String>,
    failed_tests: Vec<String>,
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
    pub fn load(&mut self, testcase: &String) {
        info!(" --------------- load testcase: {} --------------- ", testcase);
        self.now = Some(testcase.into());
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
                self.now.take();
            },
            _ => {
                info!(" --------------- TEST FAILED, exit code = {} --------------- ", exit_code);
                self.failed_tests.push(self.now.take().unwrap());
            },
        }
    }

    /// 最终输出测试信息
    pub fn final_info(&self) {
        info!(" --------------- all test ended, passed {} / {} --------------- ", self.passed, self.cnt);
        info!(" --------------- failed tests: --------------- ");
        for test in &self.failed_tests {
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
    "lmbench_all",
];

pub const BUSYBOX_TESTCASES: &[&str] = &[
    "busybox sh ./busybox_testcode.sh", //最终测例，它包含了下面全部
    "busybox echo \"#### independent command test\"",
    "busybox ash -c exit",
    "busybox sh -c exit",
    "busybox basename /aaa/bbb",
    "busybox cal",
    "busybox clear",
    "busybox date",
    "busybox df",
    "busybox dirname /aaa/bbb",
    "busybox dmesg",
    "busybox du",
    "busybox expr 1 + 1",
    "busybox false",
    "busybox true",
    "busybox which ls",
    "busybox uname",
    "busybox uptime",
    "busybox printf \"abc\n\"",
    "busybox ps",
    "busybox pwd",
    "busybox free",
    "busybox hwclock",
    "busybox kill 10",
    "busybox ls",
    "busybox sleep 1",
    "busybox echo \"#### file opration test\"",
    "busybox touch test.txt",
    "busybox echo \"hello world\" > test.txt",
    "busybox cat test.txt",
    "busybox cut -c 3 test.txt",
    "busybox od test.txt",
    "busybox head test.txt",
    "busybox tail test.txt",
    //"busybox hexdump -C test.txt", // 会要求标准输入，不方便自动测试
    "busybox md5sum test.txt",
    "busybox echo \"ccccccc\" >> test.txt",
    "busybox echo \"bbbbbbb\" >> test.txt",
    "busybox echo \"aaaaaaa\" >> test.txt",
    "busybox echo \"2222222\" >> test.txt",
    "busybox echo \"1111111\" >> test.txt",
    "busybox echo \"bbbbbbb\" >> test.txt",
    "busybox sort test.txt | ./busybox uniq",
    "busybox stat test.txt",
    "busybox strings test.txt",
    "busybox wc test.txt",
    "busybox [ -f test.txt ]",
    "busybox more test.txt",
    "busybox rm test.txt",
    "busybox mkdir test_dir",
    "busybox mv test_dir test",
    "busybox rmdir test",
    "busybox grep hello busybox_cmd.txt",
    "busybox cp busybox_cmd.txt busybox_cmd.bak",
    "busybox rm busybox_cmd.bak",
    "busybox find -name \"busybox_cmd.txt\"",

];
pub const LUA_TESTCASES: &[&str] = &[
    "lua date.lua",
    "lua file_io.lua",
    "lua max_min.lua",
    "lua random.lua",
    "lua remove.lua",
    "lua round_num.lua",
    "lua sin30.lua",
    "lua sort.lua",
    "lua strings.lua",
];

pub const DYNAMIC_TESTCASES: &[&str] = &[
    "dyn/getpwnam_r_crash.dout",
    "dyn/fflush_exit.dout",
    "dyn/tls_local_exec.dout",
    "dyn/inet_ntop_v4mapped.dout",
    "dyn/mkstemp_failure.dout",
    "dyn/utime.dout",
    "dyn/setjmp.dout",
    "dyn/string_memset.dout",
    "dyn/time.dout",
    "dyn/pthread_cond_smasher.dout",
    "dyn/fgetwc_buffering.dout",
    "dyn/pthread_rwlock_ebusy.dout",
    "dyn/sscanf_long.dout",
    "dyn/strptime.dout",
    "dyn/dn_expand_empty.dout",
    "dyn/wcsstr.dout",
    "dyn/search_tsearch.dout",
    "dyn/memmem_oob_read.dout",
    "dyn/mbc.dout",
    "dyn/basename.dout",
    "dyn/lrand48_signextend.dout",
    "dyn/regex_negated_range.dout",
    "dyn/sigprocmask_internal.dout",
    "dyn/string.dout",
    "dyn/pthread_cancel.dout",
    "dyn/crypt.dout",
    "dyn/search_hsearch.dout",
    "dyn/clocale_mbfuncs.dout",
    "dyn/regex_bracket_icase.dout",
    "dyn/snprintf.dout",
    "dyn/strverscmp.dout",
    "dyn/sem_init.dout",
    "dyn/random.dout",
    "dyn/strtold.dout",
    "dyn/iswspace_null.dout",
    "dyn/regex_ere_backref.dout",
    "dyn/tls_get_new_dtv.dout",
    "dyn/ftello_unflushed_append.dout",
    "dyn/pthread_tsd.dout",
    "dyn/pthread_exit_cancel.dout",
    "dyn/string_strchr.dout",
    "dyn/printf_fmt_g_zeros.dout",
    "dyn/daemon_failure.dout",
    "dyn/mbsrtowcs_overflow.dout",
    "dyn/strtod_simple.dout",
    "dyn/inet_pton_empty_last_field.dout",
    "dyn/strtol.dout",
    "dyn/fscanf.dout",
    "dyn/tgmath.dout",
    "dyn/ungetc.dout",
    "dyn/dn_expand_ptr_0.dout",
    "dyn/socket.dout",
    "dyn/wcsncpy_read_overflow.dout",
    "dyn/getpwnam_r_errno.dout",
    "dyn/argv.dout",
    "dyn/fpclassify_invalid_ld80.dout",
    "dyn/string_memcpy.dout",
    "dyn/setvbuf_unget.dout",
    "dyn/putenv_doublefree.dout",
    "dyn/pthread_cancel_points.dout",
    "dyn/search_insque.dout",
    "dyn/scanf_bytes_consumed.dout",
    "dyn/dirname.dout",
    "dyn/string_strcspn.dout",
    "dyn/clock_gettime.dout",
    "dyn/wcstol.dout",
    "dyn/fdopen.dout",
    "dyn/scanf_match_literal_eof.dout",
    "dyn/sscanf_eof.dout",
    "dyn/pthread_once_deadlock.dout",
    "dyn/fwscanf.dout",
    "dyn/env.dout",
    "dyn/mkdtemp_failure.dout",
    "dyn/fnmatch.dout",
    "dyn/strftime.dout",
    "dyn/wcsstr_false_negative.dout",
    "dyn/syscall_sign_extend.dout",
    "dyn/swprintf.dout",
    "dyn/tls_init.dout",
    "dyn/regexec_nosub.dout",
    "dyn/string_strstr.dout",
    "dyn/scanf_nullbyte_char.dout",
    "dyn/regex_escaped_high_byte.dout",
    "dyn/printf_fmt_g_round.dout",
    "dyn/pthread_cond.dout",
    "dyn/stat.dout",
    "dyn/sscanf.dout",
    "dyn/dlopen.dout",
    "dyn/printf_fmt_n.dout",
    "dyn/uselocale_0.dout",
    "dyn/regex_backref_0.dout",
    "dyn/qsort.dout",
    "dyn/pthread_condattr_setclock.dout",
    "dyn/inet_pton.dout",
    "dyn/search_lsearch.dout",
    "dyn/strtod.dout",
    "dyn/memmem_oob.dout",
    "dyn/string_memmem.dout",
    "dyn/fgets_eof.dout",
    "dyn/rlimit_open_files.dout",
    "dyn/strtof.dout",
    "dyn/memstream.dout",
    "dyn/udiv.dout",
    "dyn/malloc_0.dout",
    "dyn/flockfile_list.dout",
    "dyn/printf_1e9_oob.dout",
    "dyn/pthread_robust_detach.dout",
    "dyn/rewind_clear_error.dout",
    "dyn/iconv_roundtrips.dout",
    "dyn/lseek_large.dout",
    "dyn/statvfs.dout",
    "dyn/iconv_open.dout",
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
    //"sscanf_long",
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