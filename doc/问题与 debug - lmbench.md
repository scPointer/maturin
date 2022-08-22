# 问题与解决 - lmbench

### 0x01

lmbench_all 有 .rela.dyn 段，但是没有 .dynsym 段

通过 file 命令检查发现它是静态编译的，所以在这种情况下视为不加载动态库(之前默认是只要有 .rela.dyn 一定有 .dynsym)

但发现它还另外有 .symtab 等段，暂不清楚是否需要处理

处理后，发现 lmbench 在访问地址 0x28，在查找可能的原因，似乎和 random bytes 有关

出现上述问题时地址在 b5ad4，指令 `sw a4, 0(a5)`

上面a5是来自 b5aca的指令 ld
`a5,-1990(a5);` 之后是 add a5, a5,
tp

执行ld之前， a5=0xfbac6，ld的取值地址为 fb300。此处注释为 _GLOBAL_OFFSET_TABLE_+0x2e8

执行后得到 0x28，这就是报错的地址了。

发现 fb300 这个固定地址在代码中出现了300+次，再仔细翻找发现它其实是：

```asm6502
000000000006b012<__errno_location>:
__errno_location():
   6b012:   00090517             auipc a0,0x90
   6b016:   2ee53503            ld a0,750(a0) # fb300
<_GLOBAL_OFFSET_TABLE_+0x2e8>

   6b01a:   9512                 add   a0,a0,tp

   6b01c:   8082                 ret
```

查 musl 库，发现对应在 `/src/errno/__errno_location.c`

```c
int *__errno_location(void)

{

    return &__pthread_self()->errno_val;

}
```

这个函数返回的是某个结构体(其实就是tls)的某一项的地址。而内核报错的位置虽然没有调用这个函数，但用法和它类似：取出 0xfb300 处的值，加上 tp，然后写回去。

再查发现在 `/src/internal/pthread_impl.h` 有

```c
#define __pthread_self() ((pthread_t)(__get_tp() - sizeof(struct __pthread) - TP_OFFSET))
```

其中 TP_OFFSET 是 0，而在 arch/riscv64/thread_arch.h 中有

```c
static inline uintptr_t __get_tp()

{

    uintptr_t tp;

    __asm__ __volatile__("mv %0, tp" : "=r"(tp));

    return tp;

}
```

也就是获取 tp 寄存器值。

得出结论：tp 寄存器里存的是 tls 的地址，但实际上用户(libc库)读到时却是0，导致了报错。

### 0x02

之前的报错是在libc启动时调用`sys_brk`时出现的。查`https://man7.org/linux/man-pages/man2/brk.2.html`发现是 brk 的实现有问题，之前出赛时给的定义和 Linux 中的实际定义有区别。修改 sys_brk 后通过了 libc 启动阶段。

之前的问题可以概括为在启动早期的 sys_brk 不能失败，但lmbench起来之后它会自己设好 tp，就不需要关心tls的问题了。

**在整个启动过程中，libc获取syscall返回的 errno 都使用上述tls中的结构来实现，但它还没有初始化tls的时候就已经调用了sys_brk，此时没有其他机制来检测这个syscall的返回值，它一旦返回负数，libc就会以奇怪的方式退出。这是否属于是libc的问题呢？**

### 0x03

启动之后又访问了地址`0x1`，debug了一圈没有结果，又找了几个支持过lmbench的OS测试，发现是需要在 loader 是加入辅助向量 `AT_RANDOM`，并指定一个 16 Byte 的随机串的地址。

而之前 loader 对 args / envs / auxv 的处理是先加加减减统计数量，然后再一起堆到栈上。但`AT_RANDOM`的值其实也需要放在栈上，也就是说只有把这个量放上去才知道值应该写多少。基于这个原因，又修改了 loader 模块，加上了压栈时特殊处理 `AT_RANDOM`的功能。

### 0x04

跑 lmbench 发现 lat_pagefault 有时不给出输出，对应的是测例 lat_pagefault.c 中的这段代码 

```c
    benchmp(initialize, benchmark_mmap, cleanup, 0, parallel, 
        warmup, repetitions, &state);
    t_mmap = gettime() / (double)get_n();

    benchmp(initialize, benchmark, cleanup, 0, parallel, 
        warmup, repetitions, &state);
    t_combined = gettime() / (double)get_n();
    settime(get_n() * (t_combined - t_mmap));

    sprintf(buf, "Pagefaults on %s", state.file);
    micro(buf, state.npages * get_n());
```

gdb 发现给不出输出是因为判定 t_combined - t_mmap 小于 0，于是在 settime 里时间被判定成0，所以没有给出输出。（有时又能给出输出，显示是一个非常小的时间值，比单独的一个syscall还要快百倍。这个结果是不合理的，但本文档写作时榜单上已经出现了Pagefaults on /var/tmp/XXX 显示延迟为0.0x微秒的情况）

 t_combined 测的是 mmap + 随机访问造成 page fault 的时间，t_mmap 测的是仅 mmap 的时间，前者不可能更小，肯定是哪里出了问题。

继续gdb发现，在上面代码的第二段 benchmp 执行前后 t_mmap 的值变了。没有指令操作它，它是如何改变的？

查lmbench_all的反汇编，发现上面这段的代码对应

```asm6502
    1818e:    06a440ef              jal    ra,5c1f8 <benchmp>
   18192:    249420ef              jal    ra,5abda <usecs_spent>
   18196:    842a                    mv    s0,a0
   18198:    09c420ef              jal    ra,5a234 <get_n>
   1819c:    d2347453              fcvt.d.lu    fs0,s0
   181a0:    d23574d3              fcvt.d.lu    fs1,a0
   181a4:    1a9474d3              fdiv.d    fs1,fs0,fs1
   181a8:    88a6                    mv    a7,s1
   181aa:    8866                    mv    a6,s9
   181ac:    87e2                    mv    a5,s8
   181ae:    8756                    mv    a4,s5
   181b0:    4681                    li    a3,0
   181b2:    00000617              auipc    a2,0x0
   181b6:    cda60613              addi    a2,a2,-806 # 17e8c <cleanup>
   181ba:    00000597              auipc    a1,0x0
   181be:    bf058593              addi    a1,a1,-1040 # 17daa <benchmark>
   181c2:    00000517              auipc    a0,0x0
   181c6:    cf850513              addi    a0,a0,-776 # 17eba <initialize>
   181ca:    02e440ef              jal    ra,5c1f8 <benchmp>
   181ce:    20d420ef              jal    ra,5abda <usecs_spent>
   181d2:    842a                    mv    s0,a0
   181d4:    060420ef              jal    ra,5a234 <get_n>
   181d8:    d2347453              fcvt.d.lu    fs0,s0
   181dc:    d23577d3              fcvt.d.lu    fa5,a0
   181e0:    1af47453              fdiv.d    fs0,fs0,fa5
   181e4:    050420ef              jal    ra,5a234 <get_n>
   181e8:    d23577d3              fcvt.d.lu    fa5,a0
   181ec:    0a947453              fsub.d    fs0,fs0,fs1
   181f0:    1287f453              fmul.d    fs0,fa5,fs0
   181f4:    c2341553              fcvt.lu.d    a0,fs0,rtz
   181f8:    046420ef              jal    ra,5a23e <settime>
```

注意 `181a4`处得到的 fs1，它是t_mmap，这个量下一次用到是在`181ca`的 benchmp 之后的`181ec`。通过 gdb 很容易可以发现在这段没有操作 fs1的时间里，它的值变了。由此推断可能是进程切换了而 Maturin 内核没有保存浮点上下文，导致其他线程重写了 fs1 寄存器。

又查 RISCV 手册[ISA Resources | Five EmbedDev](https://five-embeddev.com/riscv-isa-manual/)，确认这个 fs1 确实是该被调用者保存的，但 benchmp 入口处只保存了 fs0 没有保存 fs1（这里插一句，不清楚这一版lmbench_all编译时发生了什么事，赛事支持方说TA也是取预编译的版本，而我之后尝试编译lmbench 也没有编译出过和它一样的 lmbench_all 文件，反汇编总会有出入）

确认了问题由来，尝试在内核中 trap 时保存浮点寄存器的上下文，又发现编译目标 riscv64imac 不支持浮点，而这就是比赛所要求的。尽管内核不能带浮点指令编译，但用户程序是已编译好的二进制文件，它们跑在支持浮点的机器上是没有问题的。

于是考虑把浮点指令硬编码进内核中：

- 更换编译目标为 riscv64gc-...

- 保存浮点寄存器上下文后编译，再反汇编得到浮点代码

- 换回riscv64imac，通过.short 形式插入硬编码的浮点指令

操作后确实可以得到正确测例结果。
