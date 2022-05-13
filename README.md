# maturin

An SMP OS written in Rust.

## Usage

```bash
$ rustup component add rust-src llvm-tools-preview
$ rustup target add riscv64imac-unknown-none-elf
$ cd kernel
$ make run
```

## Directory tree

### /user

用于测试的用户程序。部分参考了 `https://github.com/rcore-os/rCore`。

### easy-fs

目前使用的文件系统 `easy-fs` 来自 `rCore`

### rust-fatfs

一个`FAT32`格式的文件系统示例，来自 `https://github.com/rafalh/rust-fatfs`

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

### utils.rs

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
