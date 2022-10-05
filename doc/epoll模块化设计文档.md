# epoll 模块化设计文档

## 概述

- 需要拆出：`/file/epoll/`
- 可能修改：`/file/mod.rs`、`/syscall/mod.rs`、`/syscall/select.rs`
- 由于 `epoll` 模块依赖 `file`，还需要对 `file` 进行一定的模块化

## epoll 模块提供接口

- `struct EpollFile`：用作 epoll 的文件，实现 `trait File`，提供 `epoll_ctl` 等函数接口
- `bitflags EpollEventType`：epoll 事件的类型
- `struct EpollEvent`：epoll 事件
- `enum EpollCtl`：fcntl 选项

## file 模块提供接口

- 先拆出 `trait File` 这一抽象接口，将它模块化后，供 epoll 模块引用