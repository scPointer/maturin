//! 本地回环网络
//!

use alloc::{collections::BTreeMap, vec::Vec};
use core::cmp::min;
use lock::Mutex;

/// 本地的网络地址，即 127.0.0.1
pub const LOCAL_LOOPBACK_ADDR: u32 = 0x7f000001;

/// 端口映射
static PORT_MAP: Mutex<BTreeMap<u16, PortData>> = Mutex::new(BTreeMap::new());

/// 端口上的被发送或等待接收的数据
pub struct PortData {
    data: Mutex<Vec<u8>>,
}

impl PortData {
    /// 建立新的端口映射
    pub fn new() -> Self {
        Self {
            data: Mutex::new(Vec::new()),
        }
    }
    /// 读数据到 buf 中
    pub fn read(&self, buf: &mut [u8]) -> Option<usize> {
        let mut data = self.data.lock();
        let read_len = min(data.len(), buf.len());
        buf[..read_len].copy_from_slice(&data[..read_len]);
        *data = data.split_off(read_len);
        Some(read_len)
    }
    /// 从 buf 写入数据
    pub fn write(&self, buf: &[u8]) -> Option<usize> {
        let mut data = self.data.lock();
        data.extend_from_slice(buf);
        Some(buf.len())
    }
}

pub fn can_read(port: u16) -> Option<usize> {
    let map = PORT_MAP.lock();
    match map.get(&port) {
        Some(pd) => {
            let len = pd.data.lock().len();
            if len > 0 {
                Some(len)
            } else {
                None
            }
        }
        None => {
            // 端口没数据不可读
            None
        }
    }
}

pub fn read_from_port(port: u16, buf: &mut [u8]) -> Option<usize> {
    let map = PORT_MAP.lock();
    match map.get(&port) {
        Some(data) => {
            let len = data.read(buf);
            info!("Read len: {} from port: {}", len.unwrap_or(0), port);
            print_hex_dump(buf, 64);
            len
        }
        None => {
            // 端口还没有数据
            None
        }
    }
}

pub fn write_to_port(port: u16, buf: &[u8]) -> Option<usize> {
    info!("To write len: {:?} into port: {}", buf.len(), port);
    print_hex_dump(buf, 64);
    let mut map = PORT_MAP.lock();
    match map.get(&port) {
        Some(data) => data.write(buf),
        None => {
            // 新建端口数据
            info!("New a port {}", port);
            let port_data = PortData::new();
            let write_len = port_data.write(buf);
            map.insert(port, port_data);
            write_len
        }
    }
}

fn print_hex_dump(buf: &[u8], len: usize) {
    use alloc::string::String;

    //let mut linebuf: [char; 16] = [0 as char; 16];
    let mut linebuf = String::with_capacity(32);
    let buf_len = buf.len();
    let len = core::cmp::min(len, buf_len);

    for i in 0..len {
        if (i % 16) == 0 {
            print!("\t{:?}\nHEX DUMP: ", linebuf);
            //linebuf.fill(0 as char);
            linebuf.clear();
        }

        if i >= buf_len {
            print!(" {:02x}", 0);
        } else {
            print!(" {:02x}", buf[i]);
            //linebuf[i%16] = buf[i] as char;
            linebuf.push(buf[i] as char);
        }
    }
    print!("\t{:?}\n", linebuf);
}
