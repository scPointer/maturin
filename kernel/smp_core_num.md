目前SMP的核数涉及三个变量：
- `./Makefile` 开头的 `SMP ?=` 一行
- `./src/constants.rs` 的 `pub const LAST_CPU_ID: usize =` 一行
- `./src/arch/riscv/boot/entry.S` 的 `boot_stack` 常量大小

它们表达的意思相同，也需要同时更改。Todo: 将三个变量统一成一个
