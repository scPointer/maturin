//! 测例文件示例
//!
//! os启动后执行时，会运行这些文件

#[allow(dead_code)]
pub const TESTCASES: &[&str] = &[
    "busybox sh",

    // 运行 redis 示例（不是在这里打开，而是在 busybox sh 的终端中输入）
    //"./redis-server /redis.conf --loglevel verbose &", // 打开 redis 服务端，后台运行
    //"./redis-cli-static", // 打开 redis 客户端
    //"kill 3", // 退出 redis 客户端后，服务端的输出可能比较烦，可以在 busybox sh 的终端中杀掉服务端。
    // 因为终端的pid是2，所以按上面的步骤启动的服务端pid是3。当然如果之前开了其他程序，可能服务端就不是3了。
    // 或者也可以一开始就使用 "./redis-server /redis.conf &" 启动服务端，这样它就不会频繁输出 log。

    // 运行 gcc 示例
    //"./riscv64-linux-musl-native/bin/riscv64-linux-musl-gcc ./2.c -static", // 直接生成可执行文件
    //"./riscv64-linux-musl-native/bin/riscv64-linux-musl-gcc -S ./2.c -o 2.s -static", // 仅做刀编译
    //"./riscv64-linux-musl-native/bin/riscv64-linux-musl-gcc -c ./2.s -o 2.o -static", // 汇编阶段
    //"./riscv64-linux-musl-native/bin/riscv64-linux-musl-gcc 2.o -o a.out -static", // 链接阶段
    //"./a.out",
    //"busybox hexdump -C 2.o",

    // 测 lua 或者 busybox 的时候**不要**打开 base_info，内核输出非常多
    //"busybox sh lua_testcode.sh",     // lua 测例
    //"busybox sh busybox_testcode.sh", // busybox 测例
    //"busybox sh lmbench_testcode.sh", // lmbench 测例(见下)

    /* // 很少一点 libc 测例。完整评测见 ./file/test.rs 中，需要把其中 TESTCASES_ITER 和 TEST_STATUS 的值换掉
    // "./runtest.exe -w entry-dynamic.exe argv",
    // "./runtest.exe -w entry-dynamic.exe tls_init",
    // "./runtest.exe -w entry-dynamic.exe tls_local_exec",
    // "./runtest.exe -w entry-dynamic.exe pthread_exit_cancel",
    */

    /*
    //lmbench 1
    "lmbench_all lat_syscall -P 1 null",
    "lmbench_all lat_syscall -P 1 read",
    "lmbench_all lat_syscall -P 1 write",


    //lmbench 2
    "busybox mkdir -p /var/tmp",
    "busybox touch /var/tmp/lmbench",
    "lmbench_all lat_syscall -P 1 stat /var/tmp/lmbench",
    "lmbench_all lat_syscall -P 1 fstat /var/tmp/lmbench",
    "lmbench_all lat_syscall -P 1 open /var/tmp/lmbench",


    //lmbench 3
    "lmbench_all lat_select -n 100 -P 1 file",
    "lmbench_all lat_sig -P 1 install",
    "lmbench_all lat_sig -P 1 catch",
    "lmbench_all lat_sig -P 1 prot lat_sig",


    //lmbench 4
    "lmbench_all lat_pipe -P 1",
    "lmbench_all lat_proc -P 1 fork",
    "lmbench_all lat_proc -P 1 exec",
    "busybox cp hello /tmp",
    "lmbench_all lat_proc -P 1 shell",


    //lmbench 5
    "lmbench_all lmdd label=\"File /var/tmp/XXX write bandwidth:\" of=/var/tmp/XXX move=1m fsync=1 print=3",
    "lmbench_all lat_pagefault -P 1 /var/tmp/XXX",
    "lmbench_all lat_mmap -P 1 512k /var/tmp/XXX",


    //lmbench 6
    "busybox echo file system latency",
    "lmbench_all lat_fs /var/tmp",


    //lmbench 5.2
    "busybox echo Bandwidth measurements",
    "lmbench_all bw_pipe -P 1",
    "lmbench_all bw_file_rd -P 1 512k io_only /var/tmp/XXX",
    "lmbench_all bw_file_rd -P 1 512k open2close /var/tmp/XXX",
    "lmbench_all bw_mmap_rd -P 1 512k mmap_only /var/tmp/XXX",
    "lmbench_all bw_mmap_rd -P 1 512k open2close /var/tmp/XXX",


    //lmbench 7
    "busybox echo context switch overhead",
    "lmbench_all lat_ctx -P 1 -s 32 2 4 8 16 24 32 64 96",
    */
];
