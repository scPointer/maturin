目前SMP的核数涉及三个变量：
- `./Makefile` 开头的 `SMP ?=` 一行
- `./src/constants.rs` 的 `pub const CPU_NUM: usize =` 一行
- `./src/arch/riscv/boot/entry.S` 的 `boot_stack` 常量大小

它们表达的意思相同，也需要同时更改。Todo: 将三个变量统一成一个

修改上面的量时可能同时需要修改 `./src/constants.rs` 的 `PLATFORM_SIFIVE`，这样内核才能正确在启动时分辨 CPU 编号

类似地，内核栈大小也涉及两个变量：
- `./src/arch/riscv/boot/entry.S` 的 `boot_stack` 常量大小和 `slli t0, t0, 18` 一行的左移bit数是相对应的，它们同时也与SMP的核数相互影响

文件系统镜像的大小涉及两个变量：
- `./src/constants.rs` 中的 `FS_IMG_SIZE`
- `../fs-init/src/main.rs` 中生成文件系统镜像时，在代码里的常数
