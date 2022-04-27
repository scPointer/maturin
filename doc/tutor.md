# OS设计和代码解释

这个文档相当于“代码导读”，用于介绍OS的各个模块以及各种设计的思路，会包括目前存在的问题、未来可能的计划以及开发者的观点。

目前文档只是 `markdown` 形式，之后需要找个时间学用`sphinx-doc`之类的工具写更好看的文档。

[toc]

## OS的启动流程

这一段介绍从内核启动到开始执行用户程序的过程中发生的所有事情，但如果涉及到一些架构和idea，也会暂时跳到其他内容。

### 内核的第一条指令

整个程序的入口在 `/kernel/src/arch/riscv/boot/entry.S`，OS启动时它的地址是 `0x80200000`

> 为什么会从这里进入？
>
> 因为同文件夹下的 `linker.ld` 指定了 `ENTRY(_start)`，而 `/kernel/.cargo/config.toml` 中指定了使用这个 linker。同时 `/kernel/Makefile` 的 `qemu_args` 中指定了内核编译完成后的地址，与 linker 中的定义相同，所以 `OpenSBI` 初始化完成后会跳转到这个指定的地址。

### 从`entry.S` 开始

这段汇编做了以下几件事

- 将每个核的编号存到 tp
- 设置每个核的栈(sp)，这个栈会在内核初始化时使用。对于每个核来说，直到它启动第一个用户程序时才会把 sp 切换到任务对应的内核栈
- 开启在汇编中构造的页表。在这之后内核的虚拟地址是其物理地址 + `0xffff_ffff_0000_0000`
- 把栈和 start_kernel (rust 函数入口)从物理地址转换成虚拟地址，然后跳转到 `/kernel/src/main.rs` 中的 start_kernel() 执行

### 内核态地址空间映射的设计

在内核态，地址空间的映射关系一直都是 虚拟地址=物理地址+`0xffff_ffff_0000_0000`，**但在初始化的过程中可访问的地址和这些地址上的权限会不同**。下面依次介绍内核态地址空间映射的变化过程：

##### 1. 最开始用汇编构造的原始页表

 内核刚启动时，在 `entry.S` 构造了如下页表

```assembly
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

此时可用的物理地址是 `[0x8000_0000, 0xbfff_ffff]`，可以通过物理地址(实际上是恒等映射的虚拟地址)和虚拟地址访问，可以任意读/写/执行。

> 这里的页表看起来只有一页，而不是Sv39规定的三级页表。这是用了 RISC-V 的“大页”机制：当第一级或者第二集的某个页表项在 R/W/X 位有值时，就把它视为一个 1GB 或 2MB 的页，直接把表项内容当作物理页号；而只有 VALID=1但R=W=X=0时才会认为表项的内容是下一级页表的地址。

##### 2. 初始化过程中每个核的页表

在初始化函数 `start_kernel()` 中，每个核调用了`memory::kernel_page_table_init();` 此时内核中的地址按照`linker.ld` 中的分段设置权限，如 `rodata`段的内容是只读等等。

其中特别加了以下映射：

- 内核栈：每个核的内核栈的最后一页不写进 VmArea 和页表里，这意味着内核栈用到最后一页时会触发内核的缺页异常。这是为了不让一个核的内核栈溢出到其他核的栈里。
- `"phys_memory"` ：OS认为物理内存的空间是从 `0x80200000` 到 `PHYS_MEMORY_END`(=`0x8800_0000`，是 `constants.rs` 里的常数)，这段地址在内核态都可以直接访问。所以初始化会把除了 linker 中指定的内核代码段外的所有空间，也就是 [`kernel_end`, `PHYS_MEMORY_END`] 写进页表并赋读和写的权限。

##### 3. 每个用户程序的页表(内核态和用户态共用)

在任务管理器`TASK_MANAGER` 初始化时，会给每个用户程序初始化 VmArea 和对应页表。其中内核态的部分和上面所述相同，用户态的部分是由 `/loader` 子模块中的 `ElfLoader`读取用户程序信息后映射的。这段代码在 `/task/mod.rs` 的 `lazy_static! { pub  static ref TASK_MANAGER...` 定义中。

为了实现对用户程序以 4KB 页为粒度的权限控制，需要修改用户库的代码，在用户库中的 linker.ld 里每段加上 `ALIGN(4K)`。详见`/user/src/linker.ld`

用户程序申请的新的内存空间实际上是在`phys_memory`段里的。因为用户态访问时的虚拟地址和内核态直接访问`phys_memory`段的虚拟地址不同（后者一定有前缀 `0xffff_ffff`），所以这它们都放在 VmArea 里并不冲突。

### `main.rs`中的初始化流程

`entry.S` 的汇编代码会跳转到 `start_kernel` 函数执行接下来的启动流程。

这个函数主要包括以下步骤：

- 第一个核完成最初的初始化，这些操作整个OS只需要一次；其他核在自旋锁等待它完成。

```rust
    // 只有一个核能进入这个 if 并执行全局初始化操作
    if can_do_global_init() {
        memory::clear_bss(); // 清空 bss 段
        memory::allocator_init(); // 初始化堆分配器和页帧分配器
        mark_global_init_finished(); // 通知全局初始化已完成
    }
    // 等待第一个核执行完上面的全局初始化
    while !check_global_init_finished() {
        spin_loop();
    }
```

- 其他核各自完成自己的初始化

```rust
    memory::kernel_page_table_init(); // 构造内核态页表与 VmArea
    trap::init(); // 设置异常/中断的入口，即 stvec
    trap::enable_timer_interrupt(); // 开启时钟中断
    timer::set_next_trigger(); // 设置时钟中断频率
```

- 等待所有核完成初始化后，进行一些测试操作（目前还没影，但之后可能会测`ipi`等）

```rust
    // 这一步是为了进行那些**需要所有CPU都启动后才能进行的全局初始化操作**
    // 然而目前还没有这样的操作，所以现在这里只是用来展示无锁的原子变量操作(参见下面两个函数)
    mark_bootstrap_finish();
    wait_all_cpu_started();
    let cpu_id = arch::get_cpu_id();
    println!("I'm CPU [{}]", cpu_id);
```

- 然后开始执行用户程序

```rust
	// 全局初始化结束
    task::run_first_task();
    unreachable!();
```

目前每个核都执行同样的任务，也就是去跑用户程序，所以初始化完成后没有其他分支。不过之后 S7 核利用起来后或许可以用 `match cpu_id`的方式分几个核去做不同的事情。

特别地，函数最后的 unreachable 是因为所有任务执行完成后内核会通过 panic 退出，所以不会回到 start_kernel 中。

### 内核中原子变量的使用

在多核OS中，保证线程安全的方式一般是加锁，但有些简单操作用原子变量代替可能效果更高。此外，因为Mutex需要堆分配，所以在内核刚启动还没有堆分配器时，使用原子变量是唯一的选择。

本项目在初始化和进程调度时使用了原子变量。

#### 在初始化过程中(仅举一例)

如上面代码中全局只做一次的初始化操作是这样实现的：

```rust
/// 是否已经有核在进行全局初始化
static GLOBAL_INIT_STARTED: AtomicBool = AtomicBool::new(false);

/// 是否还没有核进行全局初始化，如是则返回 true
fn can_do_global_init() -> bool {
    GLOBAL_INIT_STARTED.compare_exchange(false, true, Ordering::Release, Ordering::Relaxed).is_ok()
}
```

> 原子变量的最重要的两个原语是 CAS 和 FAA
>
> CAS 即 compare and swap，调用者提供两个值 a,b，如果原子变量的值等于a，则会被替换为b，否则原子变量不变。返回值为**操作前**这个原子变量的值。
>
> FAA 即 fetch and add，调用者提供一个值 a，原子变量+=a。返回值为**操作前**这个原子变量的值。
>
> 需要注意的是 CAS 有成功和失败两种情况，而 FAA 一定成功，所以如果能选还是尽量用 FAA

rust 标准库中`core::sync::atomic::AtomicBool` 这个类型的 CAS 操作就是上面的 compare_exchange。

所以 `can_do_global_init()` 的含义是：

- 如果变量GLOBAL_INIT_STARTED`中的值是 false，那么将其变为 true，并返回 true
- 否则不变，并返回false

它保证了在所有核上执行这个函数时，有且仅有一个核会得到返回值 true。

##### 对比其他写法 (以 `aCore` 为例)

在其他多核 OS 中，处理全局初始化的方式一般是这样的：

```rust
// 代码来自 aCore 项目
if cpu_id == config::BOOTSTRAP_CPU_ID {
    ...// 进行全局只有一次的初始化
    ...// 进行这个核自己的初始化
    AP_CAN_INIT.store(true, Ordering::Release);
} else {
    // 等待上面的初始化完成
    while !AP_CAN_INIT.load(Ordering::Acquire) {
        spin_loop_hint();
    }
    ...// 进行这个核自己的初始化
```

这个过程也用了原子变量，Maturin OS 和它的写法实际上达到的效果是一样的，只是有一点小小的优化：

- 不需要特地指定一个用于初始化的核。我们初期在`sifive_u740`板子上调试时发现它会最开始只起一个核，而这个核不一定是 cpu0，所以这样写鲁棒性更强
- 减少重复的代码。如果用 if-else 分开第一个核与其他核的启动，那么因为除了全局初始化以外剩下的单核启动部分是相同的，所以单核启动部分的代码要在if/else里分别写一遍，不太好

本质上来说我们的方案只是让初始化代码看上去更直觉，减少冗余。

#### 在进程调度中

与初始化不同，进程调度中使用原子变量主要是为了能替换掉一些锁的时候，提高效率。不过目前只有一个小应用，还没有做到“用原子变量优化调度效率”这一步。

进程调度器 `TaskManager` 中有一个没有任何锁保护的量 `finished_core_cnt: AtomicUsize`，用来表示有多少个核已经“完成所有任务"（也就是找不到可继续运行的用户程序了，这个核只好无限 spin_loop{}）

在单核的OS中，如果没有可继续运行的用户程序，就可以退出 OS 了；但多核OS中一个核找不到新的用户程序时，可能其他核还在运行，只有最后一个退出的核才能宣布OS退出。在这种情况下，如果使用常规的 `Arc<Mutex<usize>>`就要写成类似：

```rust
if 找不到用户程序可以执行 {
    let mut cnt = self.finished_core_cnt.lock();
    *cnt += 1;
    if *cnt == CPU_NUM {
        panic!("All applications completed!");
    } else {
        drop(cnt);
    }
    loop {}
}
```

比较麻烦而且需要手动 drop 掉引用。使用原子变量就可以写成

```rust
if 找不到用户程序可以执行 {
    if self.finished_core_cnt.fetch_add(1, Ordering::Acquire) + 1 == CPU_NUM {
           panic!("All applications completed!");
    }
    loop {}
}
```

表示将`finished_core_cnt`自增1并取出这个变量之前的值，如果之前是 CPU_NUM - 1，说明当前核是最后一个完成用户程序的，那么通过`panic`退出OS；否则循环等待。

这个写法比用锁的写法更直观，开销也更小。

## 内核中的模块

### 多核下的任务调度模块 /task

### 内存管理模块 /memory

### 与架构和`sbi`相关的模块 /arch (以及 timer.rs)

### 中断异常处理模块 /trap

### 系统调用模块 /syscall

### 字符输入输出支持 console.rs

### 内核错误处理 error.rs

### 用户库与 loader

### 系统常量 constants.rs 及存在的问题

## 设计及约定

### 模块之间的引用

### 抽象化

### 外部依赖库