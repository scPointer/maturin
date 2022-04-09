# maturin

An SMP OS written in Rust.

## how to run
```bash
$ rustup component add rust-src llvm-tools-preview ## need it?
$ rustup target add riscv64imac-unknown-none-elf
$ cd kernel
$ make run
```