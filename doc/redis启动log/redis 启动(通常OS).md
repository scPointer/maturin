# redis 启动(通常OS)



### redis server 启动

按 "./redis-server /redis.conf"启动
其中配置文件内容为：

```
protected-mode no
port 6379
```

server 端的 syscall 输出大致概括如下

- mmap：仅申请空间。有两种形态 `prot=[PROT_READ | PROT_WRITE] flags=[MAP_PRIVATE | MAP_ANONYMOUS]` 和 `prot=[(empty)] flags=[MAP_PRIVATE | MAP_ANONYMOUS]`

- sigaction / sigprocmask 等：涉及 124678bdf（        
  
  SIGHUP = 1,
  
  SIGINT = 2,
  
  SIGILL = 4,
  
  SIGABRT = 6,
  
  SIGBUS = 7,
  
  SIGFPE = 8,
  
  SIGSEGV = 11,
  
  SIGPIPE = 13,
  
  SIGTERM = 15,）

- 有3个 clone：
  
  ```
  Syscall CLONE, [7d0f00, 542ef0, 542f40, 542fe8, 41d01a8, 542fe8] ...ret -> 4(0x4)
  Syscall CLONE, [7d0f00, 945ef0, 945f40, 945fe8, 41d01a8, 945fe8] ...ret -> 5(0x5)
  Syscall CLONE, [7d0f00, d48ef0, d48f40, d48fe8, 41d01a8, d48fe8] ...ret -> 6(0x6)
  ```

    创建线程

- mprotect：三句对应上面的三个线程：
  
  每次 clone 之前，先用空权限 mmap 获取一段内存，然后 mprotect 中间一段，前后各剩下一截。

- 其他 syscall：fs 和 socket 相关，详细见具体 log。其他比较有特点的有：umask / pipe / getcwd / ioctl / sysinfo / prlimit64

### 启动后的等待循环

server 会在自己的时间片反复调用 `clock_gettime / epoll_wait / futex / open` 进行检查

### redis client 连接后

由于 maturin 中没有接到 qemu 往外走的网络驱动，所以这里需要以 `./redis-server /redis.conf &` 后台启动，将输入还给 busybox shell。

同时，由于 server 端启动后有上述的等待循环，所以从这里开始把 `clock_gettime / epoll_wait / futex / open` 这四个 syscall 的输出关掉了。

- mmap：也是只有`prot=[PROT_READ | PROT_WRITE] flags=[MAP_PRIVATE | MAP_ANONYMOUS]`

- 一些常规 syscall

- 有两个 syscall 被内核忽视，直接返回0：`getpeername / madvise`

### redis client 进行KV存取

比较简单，每次操作基本就是 mmap / munmap 然后读写文件(文件已被 epoll 设置好映射)。
目前还不清楚为什么每次都要 mmap，可能是它把每次 transaction 都当成新程序去跑？

```
127.0.0.1:6379> keys *
Syscall MMAP, [0, 5000, 3, 22, ffffffffffffffff, 0]
mmap start=0 len=5000 prot=[PROT_READ | PROT_WRITE] flags=[MAP_PRIVATE | MAP_ANONYMOUS] fd=-1 offset=0
 ...ret -> 13930496(0xd49000)
Syscall READ, [7, d494ad, 4000, 0, 0, 0] ...ret -> 21(0x15)
Syscall MMAP, [0, 7000, 3, 22, ffffffffffffffff, 0]
mmap start=0 len=7000 prot=[PROT_READ | PROT_WRITE] flags=[MAP_PRIVATE | MAP_ANONYMOUS] fd=-1 offset=0
 ...ret -> 13955072(0xd4f000)
Syscall WRITEV, [7, 3fffb880, 1, 0, 0, 0] ...ret -> 4(0x4)
(empty array)
127.0.0.1:6379> Syscall MUNMAP, [d49000, 5000, 0, ffffffff80000001, ffffffff80000001, 0] ...ret -> 0(0x0)
127.0.0.1:6379>

127.0.0.1:6379> set mycsv 1,mydata,10
Syscall MMAP, [0, 5000, 3, 22, ffffffffffffffff, 0]
mmap start=0 len=5000 prot=[PROT_READ | PROT_WRITE] flags=[MAP_PRIVATE | MAP_ANONYMOUS] fd=-1 offset=0
 ...ret -> 13930496(0xd49000)
Syscall READ, [7, d494cd, 4000, 0, 0, 0] ...ret -> 42(0x2a)
Syscall MMAP, [0, 7000, 3, 22, ffffffffffffffff, 0]
mmap start=0 len=7000 prot=[PROT_READ | PROT_WRITE] flags=[MAP_PRIVATE | MAP_ANONYMOUS] fd=-1 offset=0
 ...ret -> 13983744(0xd56000)
Syscall WRITE, [7, d4e098, 5, 0, 0, 0] ...ret -> 5(0x5)
OK
127.0.0.1:6379> Syscall MUNMAP, [d49000, 5000, 0, ffffffff80000001, ffffffff80000001, 0] ...ret -> 0(0x0)
127.0.0.1:6379>
```

### redis client 进行其他操作

- `PING`：和上面一致

- `CONFIG GET *`：和上面区别不大，多了 `getcwd`

- `info`：和上面区别不大，多了 `uname` `madvise`

- `save`：将表写入 fs。
  
  - 多了一些 fs 相关的 syscall。比较特别的有 `fsync / renameat2 / fdatasync`。其中 `fdatasync` 我没有实现；`fsync` 有实现但 redis 使用时是之后直接接 `close`，所以可能不是必要的。

### 通过 kill 退出 server

这里的操作是，先通过 quit 退出 client，然后通过(busybox 给 server 发)结束进程信号退出。

这里调用了 sigreturn，此外和上述的写入 fs 的情况区别不大。
