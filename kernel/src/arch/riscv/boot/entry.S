    .section .text.entry
    .global _start
_start:
    # a0 == hartid
    # pc == 0x80200000
    mv tp, a0

    # 1. init per core boot_stack
    add t0, a0, 1
    slli t0, t0, 18
    la sp, idle_stack
    add sp, sp, t0

    # 2. enable page table
    la      t0, boot_page_table_sv39
    srli    t0, t0, 12
    li      t1, 8 << 60
    or      t0, t0, t1
    csrw    satp, t0
    sfence.vma

    # 3. jump to start_kernel
    la      t0, start_kernel
    li      t1, 0xffffffff00000000
    add     t0, t0, t1
    add     sp, sp, t1
    jr      t0

    .section .text.entry
    .global _start_secondary
_start_secondary:
    # a0 == hartid
    # pc == 0x80200000
    mv tp, a0

    # 1. init per core boot_stack
    add t0, a0, 1
    slli t0, t0, 18
    la sp, idle_stack
    add sp, sp, t0

    # 2. enable page table
    la      t0, boot_page_table_sv39
    srli    t0, t0, 12
    li      t1, 8 << 60
    or      t0, t0, t1
    csrw    satp, t0
    sfence.vma

    # 3. jump to start_kernel
    la      t0, start_kernel_secondary
    li      t1, 0xffffffff00000000
    add     t0, t0, t1
    add     sp, sp, t1
    jr      t0

    .section .data
    .align 12
boot_page_table_sv39:
    .quad 0
    .quad 0
    # 0x00000000_80000000 -> 0x80000000 (1G, VRWXAD)
    .quad (0x80000 << 10) | 0xcf
    # removed
    #.quad 0
    .zero 8 * 507
    # 0xffffffff_80000000 -> 0xffffffff_80000000 (1G, VRWXAD)
    .quad (0x80000 << 10) | 0xcf
    .quad 0

    .section .bss.stack
    .global idle_stack
    .global idle_stack_top
idle_stack:
    .space 256 * 1024 * 5    # 256 K per core * 4
idle_stack_top:
