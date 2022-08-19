# Maturin

一个内核。

## Doc

**初赛文档见<a href="doc/preliminary.md">这里</a>**

## Usage

```bash
$ cd kernel
$ make testcases-img
$ make run
```

> **注意** `qemu`版本至少应为`6.0.0`，`5.0`版本的`qemu`自带的`opensbi`的在启动时的行为不一样。

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

加载到文件系统镜像的测例不一定要全部运行，目前只默认运行十个测例。在`kernel/src/file/device/test.rs` 的 109 行左右有如下逻辑：

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
在文件下面还有常量 `BUSYBOX_TESTCASES` `LUA_TESTCASES` `LIBC_DYNAMIC_TESTCASES` `LIBX_STATIC_TESTCASES`。可以把它们中的一部分复制到 `SAMPLE` 中，也可以用这几个常量的名字**替换** `TESTCASES_ITER` 和 `TEST_STATUS` 中的 `SAMPLE`，实现快速测试整组测例。

> 需要说明的是，"每次单独测试一个测例"是比赛评测导致的，因为评测机只凭借串口输出检查测例是否通过，而且无法分辨多个测例同时输出时的情况。
> 
> 这里的本地测试通过不一定表示实际测试也能通过。因为测例是通过 `/libc` 模块下手动生成的，这样才能不依赖于比赛给定的 `runtest.exe` 和 `run-dynamic.sh` 和 `run-static.sh` 运行测例。否则，测试将全程交给这三个文件，想修改测例只能修改磁盘镜像里的文件，比改代码更麻烦。
> 
> 目前是由单核执行测例，因为文件系统还没有做多核的适配。所有相关选项在 `/kernel/src/constants.rs` 里

## Directory tree

### /user

用于测试的用户程序。部分参考了 `https://github.com/rcore-os/rCore`。

### /dependencies

直接抓下来的依赖库，为了规避评测机本地没有又连不上网的问题。

注意并不是所有依赖库都在这个目录下。经过大幅度修改、只适用于这个OS的库没有放在里面。

#### /dependencies/bitmap-allocator

一个分配器，用于页帧和pid分配。来自 `https://github.com/rcore-os/bitmap-allocator`

#### /dependencies/kernel-sync

依赖库，提供在使用时关中断的 Mutex ，来自方便在内核常开中断

#### /dependencies/core2

提供 no_std 下原来 std::io 类型的相关 Trait 实现，来自 `"https://github.com/bbqsrc/core2"`。

#### /dependencies/easy-fs

之前使用的文件系统，来自 `rCore`，是`https://github.com/rcore-os/rCore-Tutorial-v3` 的一部分。

目前已弃用。

#### /dependencies/easy-fs-fuse

配合 `easy-fs` 导入用户程序。来自 `rCore`，是`https://github.com/rcore-os/rCore-Tutorial-v3` 的一部分。

目前已弃用。

### /fscommon

文件系统在内存中的 `buffer` 层抽象。

本来应该是 `rust-fatfs` 的依赖库，来自 `https://github.com/rafalh/rust-fscommon`，但它所依赖的 [core_io](`https://github.com/jethrogb/rust-core_io`) 库限定死了 `rustc` 版本，而且已经没有再更新了，导致新版的编译库无法在 `no_std` 下 build 这个库。在 [这个issue](https://github.com/jethrogb/rust-core_io/issues/35) 里能看到同样有其他人遇到了从 `rust-fatfs` 到这个 `core_io` 的依赖库问题。
解决方案是换掉 [core_io](`https://github.com/jethrogb/rust-core_io`)，改为 [core2](`https://github.com/technocreatives/core2`)。

更换依赖库后，因为这两个库还是有一些接口上的不同，所以 `Cargo.tom;/dependencies` 和代码也需要修改。改后的项目已经和原项目不同了，因此你可以看到在 `kernel` `rust-fatfs` 的依赖中，`fscommon` 都使用相对路径。但是在 `fs-init` 中直接使用的是原版的 `fscommon`，因为它需要在 `std` 环境下运行，而修改后只支持 `no_std` 了。

`fscommon` 原项目采用 `MIT License`，修改后的项目也不变，所以可以在 `Cargo.toml` 中找到原作者和项目的信息。不过因为忘记在改这个模块的代码前 commit 一次，所以可能不太方便比较修改了哪些内容。

#### 所以 `core_io` 和 `core2` 的作用是

引入它们都是为了提供在 `no_std` 环境下类似 `std::io::{Read, Write, Seek}` 的接口。

`rust-fatfs` 需要针对 `fscommon::BufStream` 进行读写，它相当于一种缓存，本体在内存中，但会在需要的时候读写"文件"，且在 Drop 时也会自动写回"文件"。上文的文件在 `std` 环境下可以是 `std::fs::File`，但在 `no_std` 环境下，如这个OS，可以是一个块设备。
为了对不同的"文件"都能实现缓存，`fscommon::BufStream` 中对这个"文件"的要求就是实现 `std::io::{Read, Write, Seek}`。当然，`no_std` 环境下需要找一个类似的接口，如原项目的 `core_io` 和现在的 `core2`。

#### 内核为什么要关心这个接口

因为内核需要把 `MMIO` 提供的块设备接口包装成 `fscommon::BufStream` 所需要的实现了 `Read, Write, Seek` 的接口，所以内核必须先知道这三个接口来自哪里，有什么要求，才能对应实现 `Trait`。

### /rust-fatfs

一个`FAT32`格式的文件系统示例，来自 `https://github.com/rafalh/rust-fatfs`

这个文件系统本来是面向单核的，现改成了多核实现。具体来说需要 RefCell/Cell 改成 lock::Mutex、各个结构体内对文件系统本身的带生命周期的引用 `&'a FileSystem` 改为 Arc 等等，然后手动检查冲突和死锁。

### /fs-init

手写的工具，用于将用户程序加载到文件系统。

启动的完整流程是：

1. 在 `\kernel` 下编译内核；
2. 先在 `\user` 下 `make` ，生成用户程序；
3. 然后在 `\fs-init` 下 `make` ，生成文件系统镜像；
4. 最后在 `\kernel` 下启动 qemu，加载内核和文件系统的镜像

### /oscomp_testcases

OS比赛用到的测例。目前是2021年版本的，仅作为参考

### /repo

每周的进展交流

### /doc

帮助文档，定位是讲解OS设计

### /kernel/src

操作系统本体

#### main.rs

多核启动及初始化

#### loader.rs

加载用户程序

#### constants.rs

代码中用到的几乎所有常量

#### utils.rs

一些常用但跟 OS 设计关系不大的函数

#### timer.rs

时钟中断与相关寄存器

#### console.rs

`no_std` 下的 `print!` 及 `println!` 封装

#### error.rs

操作系统自己定义的错误处理

#### lang.rs

panic时的处理，主要是`panic_handler`

#### /arch

程序入口以及对其他一些内嵌汇编的封装，包括 sbi 调用

#### /trap

中断与异常处理。目前内核与用户中断处理尚未分离

#### /task

任务管理及调度

#### /syscall

系统调用处理

#### /drivers

设备和驱动

#### /file

一些满足文件要求的类(标准输入输出/管道/文件系统中的文件等)，以及每个进程管理文件描述符的 `FdManager` 类

#### /loaders

<del>从 `.elf` 文件中读取用户程序信息并生成对应的VMA</del>

目前所有用户程序在启动时被加载文件系统中。

#### /memory

页表虚拟地址空间管理

##### /memory/allocator

堆、页帧、进程号(PID)的分配，需要在启动时由且仅由一个核进行初始化

##### /memory/areas

表示一段有相同访问权限的内存区间，也负责处理区间内的缺页异常

##### /memory/page_table_impl_rv64_sv39

基于 `crate riscv` 实现的 `riscv64` 平台下`SV39`模式的页表。**目前已废弃**。

目前使用的页表在 `/memory/page_table.rs` 中，是手写的`SV39`模式的页表。
