//! 测例文件示例
//! 
//! os启动后执行时，会运行这些文件

#[allow(dead_code)]
pub const TESTCASES: &[&str] = &[
    
    // 测 lua 或者 busybox 的时候**不要**打开 base_info，内核输出非常多

    //"busybox sh lua_testcode.sh", // lua 测例，已通过
    "busybox sh busybox_testcode.sh", // busybox 测例
    //"busybox ls",

    /* // 很少一点 libc 测例。完整评测见 ./file/test/rs 中，需要把 TESTCASES_ITER 和 TEST_STATUS 的值换掉
    //"./runtest.exe -w entry-dynamic.exe argv",
    //"./runtest.exe -w entry-dynamic.exe tls_init",
    //"./runtest.exe -w entry-dynamic.exe tls_local_exec",
    //"./runtest.exe -w entry-dynamic.exe pthread_exit_cancel",
    */ 

    //"lmbench_all lat_syscall -P 1 null", // lmbench 测例，暂时还不能跑

    //"busybox", // busybox 提示信息    
];