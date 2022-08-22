//! Chapter 11. Performance Monitoring Unit Extension (EID #0x504D55 "PMU")

pub const EID_PMU: usize = crate::eid_from_str("PMU") as _;
pub use fid::*;

/// §11.11
mod fid {
    /// §11.5
    pub const PMU_NUM_COUNTERS: usize = 0;
    /// §11.6
    pub const PMU_COUNTER_GET_INFO: usize = 1;
    /// §11.7
    pub const PMU_COUNTER_CONFIG_MATCHING: usize = 2;
    /// §11.8
    pub const PMU_COUNTER_START: usize = 3;
    /// §11.9
    pub const PMU_COUNTER_STOP: usize = 4;
    /// §11.10
    pub const PMU_COUNTER_FW_READ: usize = 5;
}
