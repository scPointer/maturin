启动：arch_entry（即_main_for_arch）前面带的 rust_main 和 _start 都在 polyhal 里写了。我的内核的页表/栈/ linker 设计跟这边不太一样，导致想全盘接入 polyhal 会比较麻烦。
有没有可能 _start 改成 weak 的链接或者留个 feature 之类的选项？

我想把串口 / 开关机 / 页表 / 中断 / TrapFrame 啥的单独拆出来，把 polyhal 当成工具箱用一用，但似乎只有串口输入(DebugConsole)和页表分配(PageAlloc)部分可以。实质上是 polyhal 当了 sbi_rt 和 buddy_system_allocator 的传话筒。

其他部分或许可以拆得更细一些，但是因为一些平行依赖，目前看来还是不太行。比如页表地址、大页配置需要在 _start 用到；进程切换和 percpu 联动，需要内核默认使用 polyhal 的启动代码和 percpu 配置；......

许多特制常量直接写在 polyhal 里，比如 SIG_RETURN_ADDR 之类，没有类似 axconfig 那样的配置。所以如果我想用的话，可能会单独拉一个 polyhal 自己改，而不是通过 Cargo 引用。当然目前支持内核的解决办法似乎是在 polyhal 这边单独拉一个 commit 用来定制对应的参数。
