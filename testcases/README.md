# 测试样例说明

目录中大部分样例来自 `https://github.com/oscomp/testsuits-for-oskernel`

在 `../kernel` 中执行命令 `make testcases-img` 可以将这个目录的**某个子目录**作为“硬盘”加载到一个FAT32镜像中，存在 `../os.bin`。具体选择的子目录参见 `../kernel/Makefile` 中的 `DISK_DIR` 变量和 `testcases-img` 项。也可在 `make` 时直接指定该变量，如：

```bash
DISK_DIR=libc make testcases-img
```

**tips: 在重新生成文件系统镜像后如需要运行，需要先 `make clean`**

## 包含测例

`libc` 下包含：

- 初赛测例
- musl-libc 相关测例

`busybox` 下包含：
- `busybox` 相关测例
- `lua` 相关测例
- `lmbench` 相关测例
