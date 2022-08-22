# 问题与 debug - libc

### 0x01

发现 libc-test 的 `runtest.exe` 和 `entry-static.exe` `entry-dynamic.exe` 里有一些目前尚未支持的 syscall，跑不起这几个程序就没法测任何一个测例，但不经过测例测试又没法正确支持更高级的 syscall，看起来是死循环了......

但观察这几个程序的代码，发现它们都是简单调用了测例执行然后获取输出，有没有办法只执行测例不要上面的几个程序呢？我分析了比赛 `libc-test` 给出的 `Makefile`，它和原版 `libc-test`的`Makefile`有很大不同。对其进行修改后我重写了一个简单版的，这个版本可以直接编译测例为二进制程序，不需要上层的`runtest.exe`和`entry-static.exe entry-dynamic.exe`就可以运行。通过这个方法，第一次通过了 65/112 的测例。

update：上面提到的直接编译方法现在放在项目的 `/libc`目录下，作为一个子模块

### 0x02

尝试支持动态库。

首先尝试在本机上跑起动态库测例，发现直接编译执行好像行不通。在上面的`/libc`库直接编译的基础上，又在`Makefile`中添加了编译动态库的指令，同上加上参数  `-Wl,-rpath ./`，表示可以从当前目录获取动态库信息。这样想在本机(x86)测试测例就不需要把比赛提供的库塞进真实环境的 /bin 了

查资料发现动态库支持主要是需要在加载ELF程序时切换解释器、并加载动态符号。参考了`https://github.com/riscv-non-isa/riscv-elf-psabi-doc`对 rust 的ELF规范的要求，几乎是重写了 loader 模块，最终可以支持动态库了。

### 0x03

测例`tls_get_new_dtv`出错。查明是因为它需要库 `tls_get_new-dtv_dso.so`，会去 `/bin` `/usr/bin` 等目录下依次寻找这个库，但实际上这个库在根目录下。

解决问题之后默认链接动态库到 `/lib` 下
