# 问题和 debug - busybox

### 0x01

date.lua 试图获取时区，它会试图打开 /etc/localtime

或许手动加上这样的文件会更好。但查资料发现测例里的内容相当于 linux 的 date 指令，而这个指令如果找不到 /etc/localtime ，就会默认时间是 UTC，并不会报错。所以维持现状也可通过测例

update：后续发现有大量的测例会用到一些特定的目录和文件，它们是 Linux 定义的 “虚拟”文件，不在实际的文件系统中，需要OS去做特判处理

### 0x02

file_io.lua 会试图创建一个文件，它使用 O_CREAT 和 O_EXCL 的组合，而后者的语义是确认一定要创建文件，如果原来存在这个文件会报错。

而 remove.lua 实际依赖于 file_io.lua，因为它试图打开并删除后者创建的文件。

但是 remove.lua 内部如果没找到文件会直接 return 0，相当于不执行 file_io.lua 测例也可通过 remove.lua，但是测不到 remove 功能了。这可能是测试程序的一个小问题。

### 0x03

busybox hexdump 会显示一个文件的二进制信息，但它会要求stdin输入来“换行翻页”。

实际上输入后可以看到确实有信息，但是这样自动测试就没法自动进行了

update：<- 测例文件用读文件+管道覆盖了stdin就没有出现这种情况了
