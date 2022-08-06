//! Chapter 7. IPI Extension (EID #0x735049 "sPI: s-mode IPI")

pub const EID_SPI: usize = crate::eid_from_str("sPI") as _;
pub use fid::*;

/// §7.2
mod fid {
    /// §7.1
    pub const SEND_IPI: usize = 0;
}
