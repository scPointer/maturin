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

### /repo

每周的进展交流

### /kernel/src

操作系统本体

#### main.rs 

多核启动及初始化

#### loader.rs

加载用户程序

#### constants.rs

代码中用到的几乎所有常量

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

#### /memory

页表虚拟地址空间管理

##### /memory/allocator

堆与页帧的分配，需要在启动时由且仅由一个核进行初始化


