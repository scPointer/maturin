//! Chapter 6. Timer Extension (EID #0x54494D45 "TIME")

pub const EID_TIME: usize = crate::eid_from_str("TIME") as _;
pub use fid::*;

/// §6.2
mod fid {
    /// §6.1
    pub const SET_TIMER: usize = 0;
}
