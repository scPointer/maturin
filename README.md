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

### /kernel

操作系统本体

### /user

用于测试的用户程序。部分参考了 `https://github.com/rcore-os/rCore`。
