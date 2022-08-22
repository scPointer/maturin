环境是 Win10 宿主机 + wsl

1. 事先fu740已连好网线和电源。连接串口线，发现是 rs232-hs 串口
   到 https://ftdichip.com/drivers/ 下载安装驱动

2. fetch & merge 分支 sifive。
   在 wsl 内 sudo apt install u-boot-tools 以获得 mkimage。
   按照 https://github.com/scPointer/maturin/blob/sifive/kernel/fu740.md 操作获取 os-fu740.itb 

3. 在本机上构建tftp服务器。我这是windows所以比较麻烦，下载 https://pjo2.github.io/tftpd64/ 安装后打开，设置一个目录为 tftp 服务器目录，设置 server interfaces 为本机地址。把第二步拿到的 os-fu740.itb 拖进去

4. 把串口线连接电脑，打开putty监控串口，fu740开机（需要先试着找找具体是哪个串口）
   （9-415那台已经写好了bootcmd，预计可以直接从电脑上的tftp拖os镜像并运行）串口输出显示
   
   ```
    Using ethernet@10090000 device
    TFTP from server 192.168.50.21; our IP address is 192.168.50.3
    Filename 'os-fu740.itb'.
    Load address: 0xc0000000
    Loading: *
    ARP Retry count exceeded; starting again
    Wrong Image Format for bootm command
    ERROR: can't get kernel image!
   ```
   
   这里启动失败了

5. 发现是ip有问题，在uboot启动时按键打断，进入uboot。（如果是vscode串口控制器，需要加上换行选项，否则会没有回显）。
   然后按照 https://github.com/OSLab-zCore/OSLab-Docs/blob/main/zCore%E5%A4%9A%E6%A0%B8%E9%80%82%E9%85%8DU740%E6%96%87%E6%A1%A3.md#%E9%80%9A%E8%BF%87%E7%BD%91%E7%BB%9C%E8%B5%B7zcore%E5%A4%9A%E6%A0%B8  zcore这里的指引，连接实验室wifi，保证和fu740在同一局域网，串口发送 setenv serverip 192.168.50.x，即本机 ip，再发送 saveenv。

6. ping 发现主机可以ping板子，但板子ping不通。于是关掉防火墙..没问题了。其实也可以精细设置一下通过规则只放板子进去……

7. 重启fu740。同时监视串口。自动启动成功进入os
