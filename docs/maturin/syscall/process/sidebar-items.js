initSidebarItems({"fn":[["sys_brk","修改用户堆大小，"],["sys_clone","创建一个子任务，如成功，返回其 tid"],["sys_exec","将当前进程替换为指定用户程序。"],["sys_execve","复制当前进程"],["sys_exit","进程退出，并提供 exit_code 供 wait 等 syscall 拿取"],["sys_getegid","获取有效用户组 id，即相当于哪个用户的权限。在实现多用户权限前默认为最高权限"],["sys_geteuid","获取有效用户 id，即相当于哪个用户的权限。在实现多用户权限前默认为最高权限"],["sys_getgid","获取用户组 id。在实现多用户权限前默认为最高权限"],["sys_getpid","获取当前进程的 pid。 如果该核没有正在运行的线程，则直接 panic"],["sys_getppid","获取父进程的 pid。 如果该核没有正在运行的线程，则直接 panic"],["sys_gettid","获取当前线程的编号。 每个进程的初始线程的编号就是它的 pid"],["sys_getuid","获取用户 id。在实现多用户权限前默认为最高权限"],["sys_kill","向 pid 指定的进程发送信号。 如果进程中有多个线程，则会发送给任意一个未阻塞的线程。"],["sys_mmap","映射一段内存"],["sys_mprotect","映射一段内存"],["sys_msync","映射一段内存"],["sys_munmap","取消映射一段内存"],["sys_prlimt64","修改一些资源的限制"],["sys_set_tid_address","设置 clear_child_tid 属性并返回 tid。 这个属性会使得线程退出时发送: `futex(clear_child_tid, FUTEX_WAKE, 1, NULL, NULL, 0);`"],["sys_sigaction","改变当前进程的信号处理函数。"],["sys_sigprocmask","改变当前线程屏蔽的信号类型。"],["sys_sigreturn","从信号处理过程中返回，即恢复信号处理前的用户程序上下文。"],["sys_sysinfo","获取系统的启动时间和内存信息。 目前只支持启动时间"],["sys_tkill","向 tid 指定的线程发送信号。"],["sys_uname","获取系统信息"],["sys_wait4","等待子进程执行完成。如果它还没完成，则先切换掉"],["sys_yield","进程主动放弃时间片，立即切换到其他进程执行"],["waitpid","等待一个子进程执行完成"]]});