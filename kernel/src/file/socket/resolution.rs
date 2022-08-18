//! 地址解析。目前只有 ip 地址的解析

#[derive(Debug, Clone, Copy)]
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

pub fn get_ephemeral_port() -> u16 {
    // TODO selects non-conflict high port
    static mut EPHEMERAL_PORT: u16 = 0;
    unsafe {
        if EPHEMERAL_PORT == 0 {
            EPHEMERAL_PORT = (49152 + 0 % (65536 - 49152)) as u16;
        }
        if EPHEMERAL_PORT == 65535 {
            EPHEMERAL_PORT = 49152;
        } else {
            EPHEMERAL_PORT = EPHEMERAL_PORT + 1;
        }
        EPHEMERAL_PORT
    }
}
