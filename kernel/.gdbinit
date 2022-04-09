set confirm off
set architecture riscv:rv64
target remote 127.0.0.1:15234
symbol-file target/riscv64imac-unknown-none-elf/release/maturin
display/10i $pc
