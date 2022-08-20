//! 暂时的 socket 实现

mod loopback;
mod resolution;

use super::{File, OpenFlags};
use core::mem::size_of;
use lock::RwLock;
use loopback::{can_read, can_write, read_from_port, write_to_port, LOCAL_LOOPBACK_ADDR};
pub use resolution::IpAddr;
use resolution::{addr_resolution, get_ephemeral_port, AddrType};

/// 一个套接字
pub struct Socket {
    /// socket 对应的域
    domain: Domain,
    /// 连接类型
    stype: SocketType,
    /// 具体的连接协议
    protocol: usize,
    /// SocketInner struct to modify socket
    inner: RwLock<SocketInner>,
}
pub struct SocketInner {
    flags: OpenFlags,
    local_endpoint: Option<AddrType>,
    remote_endpoint: Option<AddrType>,
    is_listening: bool,
}

impl Socket {
    pub fn new(domain: Domain, s_type: SocketType, protocol: usize) -> Self {
        
        Self {
            domain: domain,
            stype: s_type,
            protocol: protocol,
            inner: RwLock::new(SocketInner {
                flags: OpenFlags::RDWR | OpenFlags::CLOEXEC | OpenFlags::NON_BLOCK,
                local_endpoint: None,
                remote_endpoint: None,
                is_listening: false,
            }),
        }
    }
    pub fn clonew(&self) -> Self {
        Self {
            domain: self.domain,
            stype: self.stype,
            protocol: self.protocol,
            inner: RwLock::new(SocketInner {
                flags: OpenFlags::RDWR,
                local_endpoint: self.inner.read().local_endpoint,
                remote_endpoint: self.inner.read().remote_endpoint,
                is_listening: false,
            }),
        }
    }
    pub fn set_endpoint(&self, addr: *const u8, is_remote: bool) -> Option<u16> {
        match addr_resolution(addr as *const u16) {
            AddrType::Ip(ip, mut port) => {
                if is_remote {
                    self.inner.write().remote_endpoint = Some(AddrType::Ip(ip, port));
                } else {
                    if port == 0 {
                        port = get_ephemeral_port();
                    }
                    self.inner.write().local_endpoint = Some(AddrType::Ip(ip, port));
                }
                info!("set endpoint: ip {:x}, port {}", ip, port);
                Some(port)
            }
            _ => {
                warn!("set endpoint failed !");
                None
            }
        }
    }
    pub fn set_listening(&self, listen: bool) {
        self.inner.write().is_listening = listen;
    }
}

impl File for Socket {
    /// Socket 不适用普通读写
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        self.recvfrom(buf, 0, 0, &mut 0)
    }
    /// Socket 不适用普通读写
    fn write(&self, buf: &[u8]) -> Option<usize> {
        self.sendto(buf, 0, 0)
    }
    /// socket的buffer内有值则可读
    fn ready_to_read(&self) -> bool {
        if let Some(ep) = self.inner.read().local_endpoint {
            match ep {
                AddrType::Ip(_ip, port) => {
                    if self.inner.read().is_listening {
                        //Fixme, 检测新连接
                        let port = port + 100;
                        if let Some(len) = can_read(port) {
                            info!("Port {} accept: {}", port, len);
                            return true;
                        }
                    } else if let Some(len) = can_read(port) {
                        info!("Port {} can read: {}", port, len);
                        return true;
                    }
                }
                _ => {
                    warn!("local endpoint Unknown");
                }
            }
        } else {
            warn!("local endpoint is invalid {:?}", self.inner.read().local_endpoint);
        }

        false
    }
    /// socket的buffer未满则可写
    fn ready_to_write(&self) -> bool {
        if let Some(ep) = self.inner.read().remote_endpoint {
            match ep {
                AddrType::Ip(_ip, port) => {
                    if let Some(len) = can_write(port) {
                        info!("Port {} can write: {}", port, len);
                        return true;
                    } else {
                        info!("Buffer to write is full");
                        return false;
                    }
                }
                _ => {}
            }
        }
        info!("local endpoint is invalid now {:?}", self.inner.read().remote_endpoint);
        true
    }
    /// 获取文件状态信息
    fn get_status(&self) -> OpenFlags {
        self.inner.read().flags
    }
    /// 设置fd flags
    fn set_status(&self, flags: OpenFlags) -> bool {
        info!("socket set flags: {:?}", flags);
        let fl = &mut self.inner.write().flags;
        fl.set(OpenFlags::NON_BLOCK, flags.contains(OpenFlags::NON_BLOCK));
        fl.set(OpenFlags::CLOEXEC, flags.contains(OpenFlags::CLOEXEC));
        true
    }
    /// 发送消息，当且仅当这个文件是 socket 时可用
    fn sendto(&self, buf: &[u8], flags: i32, dest_addr: usize) -> Option<usize> {
        let endpoint = if dest_addr == 0 {
            self.inner
                .read()
                .remote_endpoint
                .unwrap_or(AddrType::Unknown)
        } else {
            addr_resolution(dest_addr as *const u16)
        };
        match endpoint {
            AddrType::Ip(ip, port) => {
                info!("send to ip {:x} port {}", ip, port);
                if (ip == 0) || (ip == LOCAL_LOOPBACK_ADDR) {
                    //Fixme, 用于建立TCP连接
                    let port = if flags == 100 {
                        port + 100
                    } else { port };
                    write_to_port(port, buf)
                } else {
                    warn!("Unknown IP: {:#x}", ip);
                    None
                }
            }
            AddrType::Unknown => {
                warn!("recvfrom AddrType::Unknown");
                None
            }
        }
    }
    /// 收取消息，当且仅当这个文件是 socket 时可用
    fn recvfrom(
        &self,
        buf: &mut [u8],
        flags: i32,
        src_addr: usize,
        src_len: &mut u32,
    ) -> Option<usize> {
        let endpoint = if src_addr == 0 {
            self.inner
                .read()
                .local_endpoint
                .unwrap_or(AddrType::Unknown)
        } else {
            addr_resolution(src_addr as *const u16)
        };
        match endpoint {
            AddrType::Ip(ip, port) => {
                // 按 syscall 描述，这里需要把地址信息的长度写到用户给的 src_len 的位置
                *src_len = size_of::<IpAddr>() as u32;
                if (ip == 0) || (ip == LOCAL_LOOPBACK_ADDR) {
                    //Fixme, 用于建立TCP连接
                    let port = if flags == 100 {
                        port + 100
                    } else { port };
                    read_from_port(port, buf)
                } else {
                    warn!("Unknown IP: {:#x}", ip);
                    None
                }
            }
            AddrType::Unknown => {
                warn!("recvfrom AddrType::Unknown");
                None
            }
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
pub const SOCKET_TYPE_MASK: u32 = 0xff;
