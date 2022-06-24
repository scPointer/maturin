# 这是什么

`apt install ninja-build`

把比赛 `libc-test` 分支中的测例单独拿出来编译，然后可用 qemu-riscv64 在用户态运行测试，也可生成镜像。

## 可能需要的安装

qemu，本例是 6.1.1
在包内
```bash
mkdir build
cd build
../configure --target-list=riscv64-softmmu,riscv64-linux-user
sudo make install
```
最终会安装到`usr/local/bin`下。前者是`qemu-system-riscv64`，用于运行OS；后者是`qemu-riscv64`，用于运行用户程序

## 运行

用户态执行：

```bash
qemu-riscv64 ./src/a.out
```

实际执行：

将ELF文件放到FAT镜像中，作为 OS 的输入。

## 其他

检查文件格式
`file ./a.out`

记得musl编译的时候选一下静态

测例对应 `testsuits-for-oskernel/libc-test` 目录下的测例，不一定和原版 libc 相同。