#![no_std]

/// 错误编号
#[repr(C)]
#[derive(Debug)]
pub enum ErrorNo {
    /// 非法操作
    EPERM = -1,
    /// 找不到文件或目录
    ENOENT = -2,
    /// 找不到对应进程
    ESRCH = -3,
    /// 错误的文件描述符
    EBADF = -9,
    /// 资源暂时不可用。也可因为 futex_wait 时对应用户地址处的值与给定值不符
    EAGAIN = -11,
    /// 内存耗尽，或者没有对应的内存映射
    ENOMEM = -12,
    /// 无效地址
    EFAULT = -14,
    /// 设备或者资源被占用
    EBUSY = -16,
    /// 文件已存在
    EEXIST = -17,
    /// 不是一个目录(但要求需要是一个目录)
    ENOTDIR = -20,
    /// 是一个目录(但要求不能是)
    EISDIR = -21,
    /// 非法参数
    EINVAL = -22,
    /// fd（文件描述符）已满
    EMFILE = -24,
    /// 对文件进行了无效的 seek
    ESPIPE = -29,
    /// 超过范围。例如用户提供的buffer不够长
    ERANGE = -34,
    /// 不支持的协议
    EPFNOSUPPORT = -96,
    /// 不支持的地址
    EAFNOSUPPORT = -97,
    /// 拒绝连接
    ECONNREFUSED = -111,
}
