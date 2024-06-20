# Maturin

一个内核。
xsp modified

## Doc

**初赛文档见<a href="doc/操作系统设计赛 - 初赛文档.md">这里</a>**

**决赛阶段的文档汇总见<a href="doc/项目信息 & 目录.md">这里</a>**

## Usage

```bash
$ cd kernel
$ make testcases-img
$ make run
```

> **注意** `qemu`版本至少应为`6.0.0`，`5.0`版本的`qemu`自带的`opensbi`在启动时的行为不一样。

## 常见问题

##### make build 失败或 make run 失败

- 需要在 `\kernel` 下操作，根目录下的 `Makefile` 的内容比 `\kernel\Makefile` 少得多

- 由于 Maturin 默认编译在评测机上进行，所以编译选项是加了 `--offline` 的。如本地缺库请使用 `ONLINE=1 make build`，其他操作类似。

##### 报错 `[kernel] Panicked at src/drivers/memory/mod.rs:29 called Result::unwrap() on an Err value: CorruptedFileSystem`

- 需要检查 `\kernel\src\constants.rs` 中的常量`pub const IS_PRELOADED_FS_IMG: bool`（在72行左右），需要修改这个值为 false。

## 项目结构

- `dependencies`：部分依赖库，因为比赛评测机不联网所以需要放到项目里

- `modified_dependencies`：经过修改的依赖库，和原库的功能/接口等不一定相同

- `modules`：手动实现的内核模块，以 Rust crate 的形式，希望能用在别的 OS 中

- `doc`：项目文档和报告

- `docs`：部署的 Rust 文档，见 https://scpointer.github.io/maturin/

- `kernel`：内核代码

- `testcases`：测例程序，可打包成文件系统镜像由内核读取

- `user`：(曾经的) Rust 用户测例程序

- `tools`：其他工具

## 测例切换与执行

目前可以加载 `libc` 测例或 `busybox/lua/lmbench` 测例或前面所有测例(judge)或`gcc`库，默认为 `judge`。
可以通过如下方式切换测例

```
cd kernel
make clean
DISK_DIR=busybox make testcases-img
make run
```

也可以在 `/kernel/Makefile` 里第 12 行直接修改 `DISK_DIR ?= libc` 一项。

启动程序放在 `/kernel/src/testcases.rs`，可以修改这里来改变启动后的行为，如执行测例或打开终端等。之所以把执行的程序放到内核代码里，是因为修改内核+编译要比操作文件系统去改变镜像本身快得多。
