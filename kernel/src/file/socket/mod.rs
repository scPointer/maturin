//! 暂时的 socket 实现。只是个 buffer

mod loopback;
mod resolution;

use super::{File, OpenFlags};
use core::mem::size_of;
use lock::RwLock;
use loopback::{read_from_port, write_to_port, LOCAL_LOOPBACK_ADDR};
use resolution::{addr_resolution, get_ephemeral_port, AddrType, IpAddr};

/// 一个套接字
pub struct Socket {
    /// socket 对应的域
    _domain: Domain,
    /// 连接类型
    _s_type: SocketType,
    /// 具体的连接协议
    _protocol: usize,
    /// 打开时的选项
    flags: RwLock<OpenFlags>,
    /// Save IP Port
    endpoint: RwLock<AddrType>,
}

impl Socket {
    pub fn new(domain: Domain, s_type: SocketType, protocol: usize) -> Self {
        Self {
            _domain: domain,
            _s_type: s_type,
            _protocol: protocol,
            flags: RwLock::new(OpenFlags::RDWR),
            endpoint: RwLock::new(AddrType::Unknown),
        }
    }
    pub fn set_endpoint(&self, addr: usize) -> Option<usize> {
        match addr_resolution(addr as *const u16) {
            AddrType::Ip(ip, mut port) => {
                if port == 0 {
                    port = get_ephemeral_port();
                }
                info!("set endpoint: ip {:x}, port {}", ip, port);
                let mut ep = self.endpoint.write();
                *ep = AddrType::Ip(ip, port);
                Some(port.into())
            }
            _ => {
                warn!("set endpoint failed !");
                None
            }
        }
    }
}

impl File for Socket {
    /// Socket 不适用普通读写
    fn read(&self, _buf: &mut [u8]) -> Option<usize> {
        None
    }
    /// Socket 不适用普通读写
    fn write(&self, _buf: &[u8]) -> Option<usize> {
        None
    }
    /// 获取文件状态信息
    fn get_status(&self) -> OpenFlags {
        *self.flags.read()
    }
    /// 设置fd flags
    fn set_status(&self, flags: OpenFlags) -> bool {
        info!("socket set flags: {:?}", flags);
        let fl = &mut self.flags.write();
        fl.set(OpenFlags::NON_BLOCK, flags.contains(OpenFlags::NON_BLOCK));
        fl.set(OpenFlags::CLOEXEC, flags.contains(OpenFlags::CLOEXEC));
        true
    }
    /// 发送消息，当且仅当这个文件是 socket 时可用
    fn sendto(&self, buf: &[u8], _flags: i32, dest_addr: usize) -> Option<usize> {
        match addr_resolution(dest_addr as *const u16) {
            AddrType::Ip(ip, port) => {
                info!("send to ip {:x} port {}", ip, port);
                if ip == LOCAL_LOOPBACK_ADDR {
                    write_to_port(port, buf)
                } else {
                    None
                }
            }
            AddrType::Unknown => None,
        }
    }
    /// 收取消息，当且仅当这个文件是 socket 时可用
    fn recvfrom(
        &self,
        buf: &mut [u8],
        _flags: i32,
        src_addr: usize,
        src_len: &mut u32,
    ) -> Option<usize> {
        match addr_resolution(src_addr as *const u16) {
            AddrType::Ip(ip, port) => {
                info!("receive from ip {:x} port {}", ip, port);
                // 按 syscall 描述，这里需要把地址信息的长度写到用户给的 src_len 的位置
                *src_len = size_of::<IpAddr>() as u32;
                if ip == LOCAL_LOOPBACK_ADDR {
                    read_from_port(port, buf)
                } else {
                    None
                }
            }
            AddrType::Unknown => None,
        }
    }
}

use numeric_enum_macro::numeric_enum;
numeric_enum! {
    #[repr(usize)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    #[allow(non_camel_case_types)]
    /// Generic musl socket domain.
    pub enum Domain {
        /// Local communication
        AF_UNIX = 1,
        /// IPv4 Internet protocols
        AF_INET = 2,
    }
}
numeric_enum! {
    #[repr(usize)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    #[allow(non_camel_case_types)]
    /// Generic musl socket type.
    pub enum SocketType {
        /// Provides sequenced, reliable, two-way, connection-based byte streams.
        /// An out-of-band data transmission mechanism may be supported.
        SOCK_STREAM = 1,
        /// Supports datagrams (connectionless, unreliable messages of a fixed maximum length).
        SOCK_DGRAM = 2,
        /// Provides raw network protocol access.
        SOCK_RAW = 3,
        /// Provides a reliable datagram layer that does not guarantee ordering.
        SOCK_RDM = 4,
        /// Provides a sequenced, reliable, two-way connection-based data
        /// transmission path for datagrams of fixed maximum length;
        /// a consumer is required to read an entire packet with each input system call.
        SOCK_SEQPACKET = 5,
        /// Datagram Congestion Control Protocol socket
        SOCK_DCCP = 6,
        /// Obsolete and should not be used in new programs.
        SOCK_PACKET = 10,
        /// Set O_NONBLOCK flag on the open fd
        SOCK_NONBLOCK = 0x800,
        /// Set FD_CLOEXEC flag on the new fd
        SOCK_CLOEXEC = 0x80000,
    }
}
pub const SOCKET_TYPE_MASK: usize = 0xff;
