//! 暂时的 socket 实现。只是个 buffer

mod loopback;
mod resolution;

use super::{File, OpenFlags};
use core::mem::size_of;
use loopback::{read_from_port, write_to_port, LOCAL_LOOPBACK_ADDR};
use resolution::{addr_resolution, AddrType, IpAddr};

/// 一个套接字
pub struct Socket {
    /// socket 对应的域
    _domain: usize,
    /// 连接类型
    _s_type: usize,
    /// 具体的连接协议
    _protocol: usize,
    /// 打开时的选项
    flags: OpenFlags,
}

impl Socket {
    pub fn new(domain: usize, s_type: usize, protocol: usize) -> Self {
        Self {
            _domain: domain,
            _s_type: s_type,
            _protocol: protocol,
            flags: OpenFlags::RDWR | OpenFlags::NON_BLOCK | OpenFlags::CLOEXEC,
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
        self.flags
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
