//! 运行比赛测试

use crate::{
    constants::{NO_PARENT, ROOT_DIR},
    task::TaskControlBlock,
};
use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use lazy_static::*;
use lock::Mutex;

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
    TESTCASES_ITER.lock().next().map_or_else(
        || {
            TEST_STATUS.lock().final_info();
            None
        },
        |&user_command| {
            let mut argv: Vec<String> = user_command.split(' ').map(|s| s.into()).collect();
            let argv = argv.drain_filter(|s| s != "").collect();
            TEST_STATUS.lock().load(&user_command.into());
            Some(Arc::new(
                TaskControlBlock::from_app_name(ROOT_DIR, NO_PARENT, argv).unwrap(),
            ))
        },
    )
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
        info!(
            " --------------- load testcase: {} --------------- ",
            testcase
        );
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
            }
            _ => {
                info!(
                    " --------------- TEST FAILED, exit code = {} --------------- ",
                    exit_code
                );
                self.failed_tests.push(self.now.take().unwrap());
            }
        }
    }

    /// 最终输出测试信息
    pub fn final_info(&self) {
        info!(
            " --------------- all test ended, passed {} / {} --------------- ",
            self.passed, self.cnt
        );
        info!(" --------------- failed tests: --------------- ");
        for test in &self.failed_tests {
            info!("{}", test);
        }
        info!(" --------------- end --------------- ");
        panic!("");
    }
}

lazy_static! {
    //static ref TESTCASES_ITER: Mutex<Box<dyn Iterator<Item = &'static &'static str> + Send>> = Mutex::new(Box::new(FORMAT_LIBC_STATIC.into_iter().chain(FORMAT_LIBC_DYNAMIC.into_iter())));
    //static ref TEST_STATUS: Mutex<TestStatus> = Mutex::new(TestStatus::new(&[FORMAT_LIBC_STATIC, FORMAT_LIBC_DYNAMIC].concat()));
    static ref TESTCASES_ITER: Mutex<Box<dyn Iterator<Item = &'static &'static str> + Send>> = Mutex::new(Box::new(SAMPLE.into_iter()));
    static ref TEST_STATUS: Mutex<TestStatus> = Mutex::new(TestStatus::new(SAMPLE));
}

pub const SAMPLE: &[&str] = &[
    //"lmbench_all lat_syscall -P 1 null",
    //"busybox kill 10",
    //"sigreturn",
    //"dyn/tls_init.dout",
    "./runtest.exe -w entry-dynamic.exe argv",
    "./runtest.exe -w entry-dynamic.exe tls_init",
    //"./runtest.exe -w entry-dynamic.exe tls_local_exec",
    //"./runtest.exe -w entry-dynamic.exe pthread_exit_cancel",

    //"./runtest.exe -w entry-static.exe utime",
    //"./runtest.exe -w entry-static.exe fgetwc_buffering",
    //"./runtest.exe -w entry-static.exe pthread_cancel",
    //"./runtest.exe -w entry-static.exe pthread_cancel_points",
    //"./runtest.exe -w entry-dynamic.exe pthread_cancel_points",
    //"./runtest.exe -w entry-dynamic.exe pthread_cancel",
    // "./runtest.exe -w entry-dynamic.exe tls_get_new_dtv",
    //"./runtest.exe -w entry-static.exe pthread_cancel_sem_wait",
    //"./runtest.exe -w entry-static.exe socket",

    //"./runtest.exe -w entry-dynamic.exe fdopen",
    //"./runtest.exe -w entry-dynamic.exe fscanf",
    //"./runtest.exe -w entry-dynamic.exe fwscanf",
    //"./runtest.exe -w entry-dynamic.exe ungetc",
    //"./runtest.exe -w entry-dynamic.exe fflush_exit",
    //"./runtest.exe -w entry-dynamic.exe ftello_unflushed_append",
    //"./runtest.exe -w entry-dynamic.exe lseek_large",
    //"./runtest.exe -w entry-dynamic.exe syscall_sign_extend",
    //"./runtest.exe -w entry-dynamic.exe rlimit_open_files",
    //"./runtest.exe -w entry-dynamic.exe stat",
    //"./runtest.exe -w entry-dynamic.exe statvfs",

    //"./runtest.exe -w entry-static.exe pthread_robust_detach",
    //"./runtest.exe -w entry-static.exe pthread_cancel_sem_wait",//dead
    //"./runtest.exe -w entry-static.exe pthread_cond_smasher",
    //"./runtest.exe -w entry-static.exe pthread_condattr_setclock",
    //"./runtest.exe -w entry-static.exe pthread_exit_cancel",
    //"./runtest.exe -w entry-static.exe pthread_once_deadlock",
    //"./runtest.exe -w entry-static.exe pthread_rwlock_ebusy",
];

/// 来自busybox 的测例，每行是一个命令，除busybox 之外的是参数，按空格分隔
#[allow(dead_code)]
pub const BUSYBOX_TESTCASES: &[&str] = &[
    //"busybox sh ./busybox_testcode.sh", //最终测例，它包含了下面全部
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

/// 来自 lua 的测例，每行是一个命令。lua 本身是执行程序，后面的文件名实际上是参数
#[allow(dead_code)]
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

/// 来自 libc 的动态测例
#[allow(dead_code)]
pub const LIBC_DYNAMIC_TESTCASES: &[&str] = &[
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
    "dyn/printf_1e9_oob.dout",
    "dyn/pthread_robust_detach.dout",
    "dyn/rewind_clear_error.dout",
    "dyn/iconv_roundtrips.dout",
    "dyn/lseek_large.dout",
    "dyn/statvfs.dout",
    "dyn/iconv_open.dout",
];

/// 来自 libc 的静态测例
#[allow(dead_code)]
pub const LIBX_STATIC_TESTCASES: &[&str] = &[
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
#[allow(dead_code)]
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

#[allow(dead_code)]
pub const FORMAT_LIBC_STATIC: &[&str] = &[
    "./runtest.exe -w entry-static.exe argv",
    "./runtest.exe -w entry-static.exe basename",
    "./runtest.exe -w entry-static.exe clocale_mbfuncs",
    "./runtest.exe -w entry-static.exe clock_gettime",
    "./runtest.exe -w entry-static.exe crypt",
    "./runtest.exe -w entry-static.exe dirname",
    "./runtest.exe -w entry-static.exe env",
    "./runtest.exe -w entry-static.exe fdopen",
    "./runtest.exe -w entry-static.exe fnmatch",
    "./runtest.exe -w entry-static.exe fscanf",
    "./runtest.exe -w entry-static.exe fwscanf",
    "./runtest.exe -w entry-static.exe iconv_open",
    "./runtest.exe -w entry-static.exe inet_pton",
    "./runtest.exe -w entry-static.exe mbc",
    "./runtest.exe -w entry-static.exe memstream",
    "./runtest.exe -w entry-static.exe pthread_cancel_points",
    "./runtest.exe -w entry-static.exe pthread_cancel",
    "./runtest.exe -w entry-static.exe pthread_cond",
    "./runtest.exe -w entry-static.exe pthread_tsd",
    "./runtest.exe -w entry-static.exe qsort",
    "./runtest.exe -w entry-static.exe random",
    "./runtest.exe -w entry-static.exe search_hsearch",
    "./runtest.exe -w entry-static.exe search_insque",
    "./runtest.exe -w entry-static.exe search_lsearch",
    "./runtest.exe -w entry-static.exe search_tsearch",
    "./runtest.exe -w entry-static.exe setjmp",
    "./runtest.exe -w entry-static.exe snprintf",
    "./runtest.exe -w entry-static.exe socket",
    "./runtest.exe -w entry-static.exe sscanf",
    "./runtest.exe -w entry-static.exe sscanf_long",
    "./runtest.exe -w entry-static.exe stat",
    "./runtest.exe -w entry-static.exe strftime",
    "./runtest.exe -w entry-static.exe string",
    "./runtest.exe -w entry-static.exe string_memcpy",
    "./runtest.exe -w entry-static.exe string_memmem",
    "./runtest.exe -w entry-static.exe string_memset",
    "./runtest.exe -w entry-static.exe string_strchr",
    "./runtest.exe -w entry-static.exe string_strcspn",
    "./runtest.exe -w entry-static.exe string_strstr",
    "./runtest.exe -w entry-static.exe strptime",
    "./runtest.exe -w entry-static.exe strtod",
    "./runtest.exe -w entry-static.exe strtod_simple",
    "./runtest.exe -w entry-static.exe strtof",
    "./runtest.exe -w entry-static.exe strtol",
    "./runtest.exe -w entry-static.exe strtold",
    "./runtest.exe -w entry-static.exe swprintf",
    "./runtest.exe -w entry-static.exe tgmath",
    "./runtest.exe -w entry-static.exe time",
    "./runtest.exe -w entry-static.exe tls_align",
    "./runtest.exe -w entry-static.exe udiv",
    "./runtest.exe -w entry-static.exe ungetc",
    "./runtest.exe -w entry-static.exe utime",
    "./runtest.exe -w entry-static.exe wcsstr",
    "./runtest.exe -w entry-static.exe wcstol",
    "./runtest.exe -w entry-static.exe pleval",
    "./runtest.exe -w entry-static.exe daemon_failure",
    "./runtest.exe -w entry-static.exe dn_expand_empty",
    "./runtest.exe -w entry-static.exe dn_expand_ptr_0",
    "./runtest.exe -w entry-static.exe fflush_exit",
    "./runtest.exe -w entry-static.exe fgets_eof",
    "./runtest.exe -w entry-static.exe fgetwc_buffering",
    "./runtest.exe -w entry-static.exe fpclassify_invalid_ld80",
    "./runtest.exe -w entry-static.exe ftello_unflushed_append",
    "./runtest.exe -w entry-static.exe getpwnam_r_crash",
    "./runtest.exe -w entry-static.exe getpwnam_r_errno",
    "./runtest.exe -w entry-static.exe iconv_roundtrips",
    "./runtest.exe -w entry-static.exe inet_ntop_v4mapped",
    "./runtest.exe -w entry-static.exe inet_pton_empty_last_field",
    "./runtest.exe -w entry-static.exe iswspace_null",
    "./runtest.exe -w entry-static.exe lrand48_signextend",
    "./runtest.exe -w entry-static.exe lseek_large",
    "./runtest.exe -w entry-static.exe malloc_0",
    "./runtest.exe -w entry-static.exe mbsrtowcs_overflow",
    "./runtest.exe -w entry-static.exe memmem_oob_read",
    "./runtest.exe -w entry-static.exe memmem_oob",
    "./runtest.exe -w entry-static.exe mkdtemp_failure",
    "./runtest.exe -w entry-static.exe mkstemp_failure",
    "./runtest.exe -w entry-static.exe printf_1e9_oob",
    "./runtest.exe -w entry-static.exe printf_fmt_g_round",
    "./runtest.exe -w entry-static.exe printf_fmt_g_zeros",
    "./runtest.exe -w entry-static.exe printf_fmt_n",
    "./runtest.exe -w entry-static.exe pthread_robust_detach",
    "./runtest.exe -w entry-static.exe pthread_cancel_sem_wait",
    "./runtest.exe -w entry-static.exe pthread_cond_smasher",
    "./runtest.exe -w entry-static.exe pthread_condattr_setclock",
    "./runtest.exe -w entry-static.exe pthread_exit_cancel",
    "./runtest.exe -w entry-static.exe pthread_once_deadlock",
    "./runtest.exe -w entry-static.exe pthread_rwlock_ebusy",
    "./runtest.exe -w entry-static.exe putenv_doublefree",
    "./runtest.exe -w entry-static.exe regex_backref_0",
    "./runtest.exe -w entry-static.exe regex_bracket_icase",
    "./runtest.exe -w entry-static.exe regex_ere_backref",
    "./runtest.exe -w entry-static.exe regex_escaped_high_byte",
    "./runtest.exe -w entry-static.exe regex_negated_range",
    "./runtest.exe -w entry-static.exe regexec_nosub",
    "./runtest.exe -w entry-static.exe rewind_clear_error",
    "./runtest.exe -w entry-static.exe rlimit_open_files",
    "./runtest.exe -w entry-static.exe scanf_bytes_consumed",
    "./runtest.exe -w entry-static.exe scanf_match_literal_eof",
    "./runtest.exe -w entry-static.exe scanf_nullbyte_char",
    "./runtest.exe -w entry-static.exe setvbuf_unget",
    "./runtest.exe -w entry-static.exe sigprocmask_internal",
    "./runtest.exe -w entry-static.exe sscanf_eof",
    "./runtest.exe -w entry-static.exe statvfs",
    "./runtest.exe -w entry-static.exe strverscmp",
    "./runtest.exe -w entry-static.exe syscall_sign_extend",
    "./runtest.exe -w entry-static.exe uselocale_0",
    "./runtest.exe -w entry-static.exe wcsncpy_read_overflow",
    "./runtest.exe -w entry-static.exe wcsstr_false_negative",
];

#[allow(dead_code)]
pub const FORMAT_LIBC_DYNAMIC: &[&str] = &[
    "./runtest.exe -w entry-dynamic.exe argv",
    "./runtest.exe -w entry-dynamic.exe basename",
    "./runtest.exe -w entry-dynamic.exe clocale_mbfuncs",
    "./runtest.exe -w entry-dynamic.exe clock_gettime",
    "./runtest.exe -w entry-dynamic.exe crypt",
    "./runtest.exe -w entry-dynamic.exe dirname",
    "./runtest.exe -w entry-dynamic.exe dlopen",
    "./runtest.exe -w entry-dynamic.exe env",
    "./runtest.exe -w entry-dynamic.exe fdopen",
    "./runtest.exe -w entry-dynamic.exe fnmatch",
    "./runtest.exe -w entry-dynamic.exe fscanf",
    "./runtest.exe -w entry-dynamic.exe fwscanf",
    "./runtest.exe -w entry-dynamic.exe iconv_open",
    "./runtest.exe -w entry-dynamic.exe inet_pton",
    "./runtest.exe -w entry-dynamic.exe mbc",
    "./runtest.exe -w entry-dynamic.exe memstream",
    "./runtest.exe -w entry-dynamic.exe pthread_cancel_points",
    "./runtest.exe -w entry-dynamic.exe pthread_cancel",
    "./runtest.exe -w entry-dynamic.exe pthread_cond",
    "./runtest.exe -w entry-dynamic.exe pthread_tsd",
    "./runtest.exe -w entry-dynamic.exe qsort",
    "./runtest.exe -w entry-dynamic.exe random",
    "./runtest.exe -w entry-dynamic.exe search_hsearch",
    "./runtest.exe -w entry-dynamic.exe search_insque",
    "./runtest.exe -w entry-dynamic.exe search_lsearch",
    "./runtest.exe -w entry-dynamic.exe search_tsearch",
    "./runtest.exe -w entry-dynamic.exe sem_init",
    "./runtest.exe -w entry-dynamic.exe setjmp",
    "./runtest.exe -w entry-dynamic.exe snprintf",
    "./runtest.exe -w entry-dynamic.exe socket",
    "./runtest.exe -w entry-dynamic.exe sscanf",
    "./runtest.exe -w entry-dynamic.exe sscanf_long",
    "./runtest.exe -w entry-dynamic.exe stat",
    "./runtest.exe -w entry-dynamic.exe strftime",
    "./runtest.exe -w entry-dynamic.exe string",
    "./runtest.exe -w entry-dynamic.exe string_memcpy",
    "./runtest.exe -w entry-dynamic.exe string_memmem",
    "./runtest.exe -w entry-dynamic.exe string_memset",
    "./runtest.exe -w entry-dynamic.exe string_strchr",
    "./runtest.exe -w entry-dynamic.exe string_strcspn",
    "./runtest.exe -w entry-dynamic.exe string_strstr",
    "./runtest.exe -w entry-dynamic.exe strptime",
    "./runtest.exe -w entry-dynamic.exe strtod",
    "./runtest.exe -w entry-dynamic.exe strtod_simple",
    "./runtest.exe -w entry-dynamic.exe strtof",
    "./runtest.exe -w entry-dynamic.exe strtol",
    "./runtest.exe -w entry-dynamic.exe strtold",
    "./runtest.exe -w entry-dynamic.exe swprintf",
    "./runtest.exe -w entry-dynamic.exe tgmath",
    "./runtest.exe -w entry-dynamic.exe time",
    "./runtest.exe -w entry-dynamic.exe tls_init",
    "./runtest.exe -w entry-dynamic.exe tls_local_exec",
    "./runtest.exe -w entry-dynamic.exe udiv",
    "./runtest.exe -w entry-dynamic.exe ungetc",
    "./runtest.exe -w entry-dynamic.exe utime",
    "./runtest.exe -w entry-dynamic.exe wcsstr",
    "./runtest.exe -w entry-dynamic.exe wcstol",
    "./runtest.exe -w entry-dynamic.exe daemon_failure",
    "./runtest.exe -w entry-dynamic.exe dn_expand_empty",
    "./runtest.exe -w entry-dynamic.exe dn_expand_ptr_0",
    "./runtest.exe -w entry-dynamic.exe fflush_exit",
    "./runtest.exe -w entry-dynamic.exe fgets_eof",
    "./runtest.exe -w entry-dynamic.exe fgetwc_buffering",
    "./runtest.exe -w entry-dynamic.exe fpclassify_invalid_ld80",
    "./runtest.exe -w entry-dynamic.exe ftello_unflushed_append",
    "./runtest.exe -w entry-dynamic.exe getpwnam_r_crash",
    "./runtest.exe -w entry-dynamic.exe getpwnam_r_errno",
    "./runtest.exe -w entry-dynamic.exe iconv_roundtrips",
    "./runtest.exe -w entry-dynamic.exe inet_ntop_v4mapped",
    "./runtest.exe -w entry-dynamic.exe inet_pton_empty_last_field",
    "./runtest.exe -w entry-dynamic.exe iswspace_null",
    "./runtest.exe -w entry-dynamic.exe lrand48_signextend",
    "./runtest.exe -w entry-dynamic.exe lseek_large",
    "./runtest.exe -w entry-dynamic.exe malloc_0",
    "./runtest.exe -w entry-dynamic.exe mbsrtowcs_overflow",
    "./runtest.exe -w entry-dynamic.exe memmem_oob_read",
    "./runtest.exe -w entry-dynamic.exe memmem_oob",
    "./runtest.exe -w entry-dynamic.exe mkdtemp_failure",
    "./runtest.exe -w entry-dynamic.exe mkstemp_failure",
    "./runtest.exe -w entry-dynamic.exe printf_1e9_oob",
    "./runtest.exe -w entry-dynamic.exe printf_fmt_g_round",
    "./runtest.exe -w entry-dynamic.exe printf_fmt_g_zeros",
    "./runtest.exe -w entry-dynamic.exe printf_fmt_n",
    "./runtest.exe -w entry-dynamic.exe pthread_robust_detach",
    "./runtest.exe -w entry-dynamic.exe pthread_cond_smasher",
    "./runtest.exe -w entry-dynamic.exe pthread_condattr_setclock",
    "./runtest.exe -w entry-dynamic.exe pthread_exit_cancel",
    "./runtest.exe -w entry-dynamic.exe pthread_once_deadlock",
    "./runtest.exe -w entry-dynamic.exe pthread_rwlock_ebusy",
    "./runtest.exe -w entry-dynamic.exe putenv_doublefree",
    "./runtest.exe -w entry-dynamic.exe regex_backref_0",
    "./runtest.exe -w entry-dynamic.exe regex_bracket_icase",
    "./runtest.exe -w entry-dynamic.exe regex_ere_backref",
    "./runtest.exe -w entry-dynamic.exe regex_escaped_high_byte",
    "./runtest.exe -w entry-dynamic.exe regex_negated_range",
    "./runtest.exe -w entry-dynamic.exe regexec_nosub",
    "./runtest.exe -w entry-dynamic.exe rewind_clear_error",
    "./runtest.exe -w entry-dynamic.exe rlimit_open_files",
    "./runtest.exe -w entry-dynamic.exe scanf_bytes_consumed",
    "./runtest.exe -w entry-dynamic.exe scanf_match_literal_eof",
    "./runtest.exe -w entry-dynamic.exe scanf_nullbyte_char",
    "./runtest.exe -w entry-dynamic.exe setvbuf_unget",
    "./runtest.exe -w entry-dynamic.exe sigprocmask_internal",
    "./runtest.exe -w entry-dynamic.exe sscanf_eof",
    "./runtest.exe -w entry-dynamic.exe statvfs",
    "./runtest.exe -w entry-dynamic.exe strverscmp",
    "./runtest.exe -w entry-dynamic.exe syscall_sign_extend",
    "./runtest.exe -w entry-dynamic.exe tls_get_new_dtv",
    "./runtest.exe -w entry-dynamic.exe uselocale_0",
    "./runtest.exe -w entry-dynamic.exe wcsncpy_read_overflow",
    "./runtest.exe -w entry-dynamic.exe wcsstr_false_negative",
];
