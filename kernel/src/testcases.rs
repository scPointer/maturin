//! 测例文件示例
//! 
//! os启动后执行时，会运行这些文件

#[allow(dead_code)]
pub const TESTCASES: &[&str] = &[
    
    // 测 lua 或者 busybox 的时候**不要**打开 base_info，内核输出非常多

    //"busybox sh lua_testcode.sh", // lua 测例，已通过
    //"busybox sh busybox_testcode.sh", // busybox 测例
    //"busybox ls",

    /* // 很少一点 libc 测例。完整评测见 ./file/test.rs 中，需要把其中 TESTCASES_ITER 和 TEST_STATUS 的值换掉
    //"./runtest.exe -w entry-dynamic.exe argv",
    //"./runtest.exe -w entry-dynamic.exe tls_init",
    //"./runtest.exe -w entry-dynamic.exe tls_local_exec",
    //"./runtest.exe -w entry-dynamic.exe pthread_exit_cancel",
    */ 

    //"lmbench_all lat_syscall -P 1 read",
    // "lmbench_all lat_syscall -P 1 null", // lmbench 测
    //"lmbench_all lat_syscall -P 1 write",
    //"lmbench_all lat_proc -P 1 exec",
    //"lmbench_all hello",
    //"lmbench_all lat_select -n 100 -P 1 file",
    //"lmbench_all lat_sig -P 1 install",
    //"lmbench_all lat_sig -P 1 catch",
    //"lmbench_all lat_sig -P 1 prot lat_sig",
    // "lmbench_all lmdd label=\"File/var/tmp/XXXwritebandwidth:\" of=/var/tmp/XXX move=645m fsync=1 print=3",
    // "lmbench_all lat_pagefault -P 1 /var/tmp/XXX",
    //"lmbench_all lat_mmap -P 1 512k /var/tmp/XXX",
    "lmbench_all lat_fs /var/tmp",
    //"lmbench_all lat_ctx -P 1 -s 32 2 4 8 16 24 32 64 96",
    //"lmbench_all bw_file_rd -P 1 512k io_only /var/tmp/XXX",
    //"lmbench_all bw_mmap_rd -P 1 512k mmap_only /var/tmp/XXX",
    //"lmbench_all bw_pipe -P 1",
    //"busybox", // busybox 提示信息
];
