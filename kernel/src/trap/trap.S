.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:

    bgtz sp, __user_trap_start

    # here is prework before kernel trap entry
    # in "__real_trap_entry", tp will be replaced by 34*8(sp), where we assumed is saved by __restore before.
    # but in kernel trap, it's NOT real cpu_id, we need to save current tp , and fetch it again in "__real_trap_entry"
    # now size of TrapContext is 37, remember to modify the offset when update TrapContext!
    sd tp, -3*8(sp)

    # now:
    # sp = kernel stack
    # sscratch = if trap from user (user stack) else ( if task is idle (0) else (kernel stack))
__real_trap_entry:
# allocate a TrapContext on kernel stack
    addi sp, sp, -37*8
    # save general-purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # save tp(x4) for L I B C T E S T
    sd tp, 4*8(sp)
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it on the kernel stack
    csrr t2, sscratch
    .short 0xae22 # fsd fs0, 280(sp)
    .short 0xb226 # fsd fs1, 288(sp)
    sd t2, 2*8(sp)
    # load cpu_id
    ld tp, 34*8(sp)
    # set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp
    call trap_handler

__restore:
    # now sp->kernel stack(after allocated), sscratch->user stack
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    # save cpu_id
    sd tp, 34*8(sp)
    # load user tp(x4)
    # for kernel, value stayed same, which means 4*8(sp) == 34*8(sp)
    ld tp, 4*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # get SPP
    andi t0, t0, 0x100
    bnez t0, __kernel_trap_end

__user_trap_end:
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    .short 0x2472 # fld fs0, 280(sp)
    .short 0x3492 # fld fs1, 288(sp)
    # release TrapContext on kernel stack
    addi sp, sp, 37*8

    csrrw sp, sscratch, sp
    # when sscratch == 0, trap is from kernel space
    # then switch sp and sscratch again
    beqz sp, __idle_kernel_trap_end
    # return to user
    sret

__user_trap_start:
    csrrw sp, sscratch, sp
    # when sscratch == 0, trap is from kernel space
    # then switch sp and sscratch again
    # now it's impossible because here is __user_trap_start, so REAL sp is user's
    # beqz sp, __idle_kernel_trap_start
    j __real_trap_entry

#__idle_kernel_trap_start:
#    csrrw sp, sscratch, sp
#    j __real_trap_entry

__idle_kernel_trap_end:
    csrrw sp, sscratch, sp
    # return to kernel
    sret

__kernel_trap_end:
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # release TrapContext on kernel stack
    addi sp, sp, 37*8
    sret