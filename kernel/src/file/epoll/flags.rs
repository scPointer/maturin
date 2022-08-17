//! epoll 的控制选项和事件类型


use bitflags::*;

#[derive(Copy, Clone)]
/// 指定一个 epoll 事件
pub struct EpollEvent {
    /// 事件类型，见下
    pub events: EpollEventType,
    /// 用户使用的数据，其实是个 enum，但内核不考虑
    pub data: u64,
}

bitflags! {
    /// Epoll 事件的类型
    pub struct EpollEventType: u32 {
        const EPOLLIN = 0x001;
        const EPOLLOUT = 0x004;
        const EPOLLERR = 0x008;
        const EPOLLHUP = 0x010;
        const EPOLLPRI = 0x002;
        const EPOLLRDNORM = 0x040;
        const EPOLLRDBAND = 0x080;
        const EPOLLWRNORM = 0x100;
        const EPOLLWRBAND= 0x200;
        const EPOLLMSG = 0x400;
        const EPOLLRDHUP = 0x2000;
        const EPOLLEXCLUSIVE = 0x1000_0000;
        const EPOLLWAKEUP = 0x2000_0000;
        const EPOLLONESHOT = 0x4000_0000;
        const EPOLLET = 0x8000_0000;
    
    }
}

numeric_enum_macro::numeric_enum! {
    #[repr(i32)]
    #[derive(Debug, Eq, PartialEq)]
    /// sys_fcntl64 使用的选项
    pub enum EpollCtl {
        /// 添加一个文件对应的事件
        ADD = 1,
        /// 删除一个文件对应的事件
        DEL = 2,
        /// 修改一个文件对应的事件
        MOD = 3,
    }
}
