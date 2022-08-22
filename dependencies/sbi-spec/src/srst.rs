//! Chapter 10. System Reset Extension (EID #0x53525354 "SRST")

pub const EID_SRST: usize = crate::eid_from_str("SRST") as _;
pub use fid::*;

pub const RESET_TYPE_SHUTDOWN: u32 = 0;
pub const RESET_TYPE_COLD_REBOOT: u32 = 1;
pub const RESET_TYPE_WARM_REBOOT: u32 = 2;

pub const RESET_REASON_NO_REASON: u32 = 0;
pub const RESET_REASON_SYSTEM_FAILURE: u32 = 1;

/// §10.2
mod fid {
    /// §10.1
    pub const SYSTEM_RESET: usize = 0;
}
