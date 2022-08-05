# Maturin Intro

> 2022/8/5 闭浩扬

Maturin 是一个可以在 `qemu` 或 `sifive-fu740` 上运行的多核OS。

## 目前的支持

主要的支持分四个部分

### musl-libc

目前已在决赛的第一阶段通过 libc-test 的 220 个测例，实现的特性有动态库加载、线程、信号、futex等。

### busybox

目前也可以支持大部分基础命令，但这块没有很详细的测例来测过，可能有更多bug。

### lua

是比赛要求。已通过。

### lmbench

是比赛要求，正在做。

## 使用方式

```bash
$ rustup component add rust-src llvm-tools-preview

$ rustup target add riscv64imac-unknown-none-elf

$ cd kernel

$ make testcases-img

$ make run
```

注意`qemu`版本至少应为`6.0.0`，`5.0`版本的`qemu`自带的`opensbi`的在启动时的行为不一样。

## 测例切换与执行

目前可以加载 `libc` 测例或 `busybox/lua/lmbench` 测例，默认为 `libc`。

可以通过如下方式切换测例

```
cd kernel

make clean

DISK_DIR=busybox make testcases-img

make run
```

也可以在 `/kernel/Makefile` 里第 12 行直接修改 `DISK_DIR ?= libc` 一项。

加载到文件系统镜像的测例不一定要全部运行。在`kernel/src/file/device/test.rs` 的 109 行左右有如下逻辑：

```rust
lazy_static! {

    static ref TESTCASES_ITER: Mutex<Iter<'static, &'static str>> = Mutex::new(SAMPLE.into_iter());

    static ref TEST_STATUS: Mutex<TestStatus> = Mutex::new(TestStatus::new(SAMPLE));

}



pub const SAMPLE: &[&str] = &[

......

];
```

程序会依次加载 `SAMPLE` 中的测例并运行，修改它的内容可以控制实际执行的测例。



需要说明的是，"每次单独测试一个测例"是比赛评测导致的，因为**评测机不给输入，只凭借串口输出检查测例是否通过，而且无法分辨多个测例同时输出时的情况**。

之后2-3天应该马上会更新一个基于 busybox 的 shell 的、可交互的模式。

## 模块介绍

这里不介绍太多代码细节，只提主要特点以及 zCore / rCore 不同的部分。

### memory 内存管理

##### allocator 分配器

所有分配器采用 bitset 上的 radix tree。实际是 `bitmap_allocator::BitAlloc...`



堆分配器比较简单，采用静态的 `LockedHeap`



只有 `tid` 的分配器，没有 `pid`的分配器。即对应内核中没有进程，只有“线程”。

##### areas 内存区间

内存区间支持 lazy alloc，但还不支持 copy on write

##### user 用户地址空间传入的指针/数据

规定了内核如何使用用户态传来的指针。

- `pub struct UserPtrUnchecked<T>(*mut T)` 就是强制类型转换

- `pub struct UserPtr<T>(UserPtrUnchecked<T>);` 会**在生成时** MemorySet 里检查地址是否合法

- `UserData<T, In>` 检查，并在生成时把用户地址处的数据到复制到内核中

- `UserData<T, Out>` 在析构时(一般应该是syscall返回前)检查，将其中的数据写入用户地址处

- `UserData<T, InOut>` 以上两者结合

- `trait IoFlag` 输入输出方式，也就是以上的 In / Out / InOut，可以类似 zCore 的 policy 用 trait 的方式实现

- `UserDataVec<T, F: IoFlag>` 处理输入是个长度不定的数列的情况

- 一些举例：
  
  `UserStr<F: IoFlag> = UserDataVec<u8, F>`
  
  `UserDataIoVec<F: IoFlag> = UserDataVec<UserStr<F>, In>` (sys_readv/writev)



> 这个部分和 zCore 的 `UserPtr` 有什么不同？
> 
> 在这里不只是包装用户态的指针，而是可以根据需要检查、复制用户态的数据，是需要 MemorySet 协助的。

这个模块本身其实也是非必须的。在 syscall 中用强制类型转换也是可以工作，但转换后显得更安全更漂亮一些

##### page_table 页表

采用了用户态和内核态同页表。主要是为了 trap 更简单

启动时页表的建立分三步：

1. 刚进入内核时，在 `entry.S` 构造了 `0xffffffff_80000000 -> 0x80000000`的映射

```nasm
    .section .data

    .align 12

boot_page_table_sv39:

    .quad 0

    .quad 0

    # 0x00000000_80000000 -> 0x80000000 (1G, VRWXAD)

    .quad (0x80000 << 10) | 0xcf

    .zero 8 * 507

    # 0xffffffff_80000000 -> 0x80000000 (1G, VRWXAD)

    .quad (0x80000 << 10) | 0xcf

    .quad 0
```

2. 内核初始化过程中，生成一个 `KERNEL_MEMORY_SET`，其中按照`linker.ld` 中的分段设置权限、每个核分配栈、添加 shadow_page等

3. 启动一个用户程序时，在顶层的最后两项直接映射到内核部分的两个1G的大页(`[0xffff_ffff_8000_0000, 0xffff_ffff_ffff_ffff]`)，然后再处理用户程序本身的地址映射。

##### vmm(MemorySet) 虚存管理

### trap 异常/中断

多存了一项 cpu_id。这是因为内核中(包括`kernel-sync`库)要用到  tp 作为 cpu 的编号，但是用户也需要用来存 tls，所以 trap 进入用户态的时候就额外把当前 cpu_id 存了下来。

目前分为用户态 / 用户程序内核态 / 空闲内核态的异常中断，但后两者还没有实际的应用场景。判断异常中断的来源是很关键的一点

- zCore 用的 `trapframe` 库，其中把判断条件定为：如果 `sscratch`的值是0，则认为是内核态，否则是用户态

```nasm
trap_entry:
    # If coming from userspace, preserve the user stack pointer and load
    # the kernel stack pointer. If we came from the kernel, sscratch
    # will contain 0, and we should continue on the current stack.
    csrrw sp, sscratch, sp
    bnez sp, trap_from_user
trap_from_kernel:
    csrr sp, sscratch
```

我的判定条件是把 sp 作为有符号整数看，用户态是低地址，相当于大于0；内核态是高地址，相当于小于0。

### task 任务管理

目前控制用户程序的基本块是 `TaskControlBlock`，它比较像线程，是程序调度的基本单位，但不存在一个进程的基本块。这是因为OS实现了更多的 clone 参数，由此使得每个TCB的地址空间、文件描述符、信号等都是可灵活重用的。

```rust
/// 任务控制块，包含一个用户程序的所有状态信息，但不包括与调度有关的信息。
/// 默认在TCB的外层对其的访问不会冲突，所以外部没有用锁保护，内部的 mutex 仅用来提供可变性
/// 
/// 目前来说，TCB外层可能是调度器或者 CpuLocal：
/// 1. 如果它在调度器里，则 Scheduler 内部不会修改它，且从 Scheduler 里取出或者放入 TCB 是由调度器外部的 Mutex 保护的；
/// 2. 如果它在 CpuLocal 里，则同时只会有一个核可以访问它，也不会冲突。
pub struct TaskControlBlock {
    /// 用户程序的内核栈，内部包含申请的内存空间
    /// 因为 struct 内部保存了页帧 Frame，所以 Drop 这个结构体时也会自动释放这段内存
    pub kernel_stack: KernelStack,
    /// 进程 id。创建任务时实际分配的是 tid 而不是 pid，所以没有对应的 Pid 结构保护
    pub pid: usize,
    /// 线程 id
    pub tid: Tid,
    /// 当退出时是否向父进程发送信号 SIGCHLD。
    /// 如果创建时带 CLONE_THREAD 选项，则不发送信号，除非它是线程组(即拥有相同pid的所有线程)中最后一个退出的线程；
    /// 否则发送信号
    pub send_sigchld_when_exit: bool,
    /// 信号量对应的一组处理函数。
    /// 因为发送信号是通过 pid/tid 查找的，因此放在 inner 中一起调用时更容易导致死锁
    pub signal_handlers: Arc<Mutex<SignalHandlers>>,
    /// 接收信号的结构。
    pub signal_receivers: Arc<Mutex<SignalReceivers>>,
    /// 任务的内存段(内含页表)，同时包括用户态和内核态
    pub vm: Arc<Mutex<MemorySet>>,
    /// 管理进程的所有文件描述符
    pub fd_manager: Arc<Mutex<FdManager>>,
    /// 任务的状态信息
    pub inner: Arc<Mutex<TaskControlBlockInner>>,
}
```

这导致很多原本对“进程”的操作根据所属的模块不同，被分成了更具体的语义。例：

- 线程A mmap 了一块空间，不存在一个“进程”使得其中的线程能看见这块空间，而是 clone 时带 CLONE_VM 参数的线程才能看见它。

- 线程A通过sigaction设了一个action，然后通过tkill发信号给B。B能收到对应的信号，但只有它和A的 SignalHandlers 是同一个时才会按 A 的设置处理信号。这当且仅当clone时设置了 CLONE_SIGHAND 参数

- pid对应的“多个线程的集合”也还是存在的，只是判定依据变成了"clone时设置了 CLONE_SIGHAND 参数"。它们之间可以有不同的地址空间、fd等，也可以相同。

- (这一条是设想)线程A通过kill发一个信号，则会首先找到对应tid的TCB，然后往它的 SignalReceivers 里塞一个信号。每个拥有这个 Receivers 的线程都可能发现这个信号，但因为 mutex 它们不会同时发现。每个抢到 mutex 锁的线程会试图比对自己的 mask，如果可以接收则会处理并删除这个信号。这符合 linux 对于"每个线程都有自己的信号掩码，但信号只会发给一个线程，异步信号任取一个没有阻塞的线程"的要求。



musl-libc 的 pthread-create 没有使用上面这些复杂灵活的处理，所以 zCore 那样不管 clone 时的 flags 混过去也是行得通的。

所以目前我们OS的思路只是提供了这样一种支持，对 musl-libc 不是必须的。



另外，分开不同的模块可以让不同的线程同时访问，但也容易死锁。目前的策略是，在处理syscall时，先按照上面TCB里定义的顺序拿所有需要的模块的锁，再进行操作。

##### kernel_stack (用户程序的)内核栈

会分配并保存一段连续的页帧，而不是像 rCore 那样直接默认在一个根据 pid 固定偏移的地址上。

##### scheduler 调度器

目前是 FIFO

##### cpu_local

一些需要以 cpu 的视角来处理的工作： run_user 循环、handle_signal、handle_page_fault、handle_zombie_task(有一个稍微麻烦一点的去死锁策略)

### signal 信号

都是比较正常的设计

### syscall 系统调用

等 /memory/user 写完会规范一下参数的传递

目前几乎所有syscall的选项都放在 /syscall/flags.rs 里了，可能需要用的时候得 ctrl+F

### loaders ELF解析和加载

处理了解释器、动态链接，有 args / auxv，但还没处理 envs。

如果 elf 里的 phdr 是0，那么不会类似 zCore 那样去问 MemorySet 要一个地址，而是固定放在 0x400_0000。因为现在是默认通过 exec 进入 elf 加载，所以用户的 MemorySet 是空的，就直接指定了。

### file 文件和各种"文件"

因为 linux 会把各种东西都当文件用，所以 trait File 的内容比较多，但除了 read/write 都有默认(为空的)实现。

##### device 对接文件系统

这个模块负责和实际的文件系统对接，包含了各种类似`./ ../`的路径处理、特殊目录文件 `/dev/... /tmp`初始化、FAT32上模拟符号链接......的操作

目前在 test.rs 中进行测试，上面已介绍过。

### constants.rs 常量约定

这个文件里主要有几个用到比较多的参数：

- `BASE_INFO`可以开关内核输出

- `KERNEL_STACK_SIZE` `KERNEL_HEAP_SIZE`：控制内核堆栈大小。如果后续需要内核里用很大的数据结构，要么调大它们，要么可以手动申请页帧存放

- `PLATFORM_SIFIVE` `IS_PRELOADED_FS_IMG`：评测时开启。一般在 qemu 上的时候不需要开

- `IS_SINGLE_CORE` 控制是否启动多核

### makefile

调试需要在一个终端里用 `make gdb-runner`，在另一个终端里用 `gdb-listener`，就可以开gdb了。

## 其他约定

在写这个OS的过程中，我尽量保持每个文件/struct/fn/enum项等的有完整中文注释，同时函数中也有足够多的行级中文注释


