//! 地址解析。目前只有 ip 地址的解析

pub enum AddrType {
    /// ip 地址和端口
    Ip(u32, u16),
    /// 未知
    Unknown,
}

#[repr(C)]
pub struct IpAddr {
    pub family: u16,
    pub port: u16,
    pub addr: u32,
}

//const FAMILY_UNIX: u16 = 1;
const FAMILY_INTERNET: u16 = 2;

pub fn addr_resolution(family_user_addr: *const u16) -> AddrType {
    let family = unsafe { *family_user_addr };
    match family {
        FAMILY_INTERNET => {
            let ip_addr = unsafe { &*(family_user_addr as *const IpAddr) };
            AddrType::Ip(u32::from_be(ip_addr.addr), u16::from_be(ip_addr.port))
        }
        _ => AddrType::Unknown,
    }
}
