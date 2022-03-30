    .section .text.entry
    .global _start
_start:
    # a0 == hartid
    # pc == 0x80200000
    mv tp, a0

    add t0, a0, 1
    slli t0, t0, 18
    la sp, boot_stack
    add sp, sp, t0

    tail start_kernel

    .section .bss.stack
    .global boot_stack
    .global boot_stack_top
boot_stack:
    .space 256 * 1024 * 4    # 256 K per core * 4
boot_stack_top:
