//! Chapter 9. Hart State Management Extension (EID #0x48534D "HSM")

pub const EID_HSM: usize = crate::eid_from_str("HSM") as _;
pub use fid::*;

pub const HART_STATE_STARTED: usize = 0;
pub const HART_STATE_STOPPED: usize = 1;
pub const HART_STATE_START_PENDING: usize = 2;
pub const HART_STATE_STOP_PENDING: usize = 3;
pub const HART_STATE_SUSPENDED: usize = 4;
pub const HART_STATE_SUSPEND_PENDING: usize = 5;
pub const HART_STATE_RESUME_PENDING: usize = 6;

pub const HART_SUSPEND_TYPE_RETENTIVE: u32 = 0;
pub const HART_SUSPEND_TYPE_NON_RETENTIVE: u32 = 0x8000_0000;

/// §9.5
mod fid {
    /// §9.1
    pub const HART_START: usize = 0;
    /// §9.2
    pub const HART_STOP: usize = 1;
    /// §9.3
    pub const HART_GET_STATUS: usize = 2;
    /// §9.4
    pub const HART_SUSPEND: usize = 3;
}
