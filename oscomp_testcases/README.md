# 测试样例说明

本目录中绝大部分样例来自 `https://github.com/oscomp/testsuits-for-oskernel`，但有如下例外：

- `start` 是OS指定的入口程序，每次启动时会先启动它作为第一个进程
- `user_shell` 是终端程序，负责启动其他用户程序并传递 `argc/argv`。它是由 `start` 通过 `fork+exec` 启动的


