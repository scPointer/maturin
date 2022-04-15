目前SMP的核数涉及三个变量：
- `./Makefile` 开头的 `SMP ?=` 一行
- `./src/constants.rs` 的 `pub const CPU_NUM: usize =` 一行
- `./src/arch/riscv/boot/entry.S` 的 `boot_stack` 常量大小

它们表达的意思相同，也需要同时更改。Todo: 将三个变量统一成一个

类似地，内核栈大小也涉及两个变量：
- `./src/arch/riscv/boot/entry.S` 的 `boot_stack` 常量大小和 `slli t0, t0, 18` 一行的左移bit数是相对应的，它们同时也与SMP的核数相互影响