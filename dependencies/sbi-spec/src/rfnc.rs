//! Chapter 8. RFENCE Extension (EID #0x52464E43 "RFNC")

pub const EID_RFNC: usize = crate::eid_from_str("RFNC") as _;
pub use fid::*;

/// §8.8
mod fid {
    /// §8.1
    pub const REMOTE_FENCE_I: usize = 0;
    /// §8.2
    pub const REMOTE_SFENCE_VMA: usize = 1;
    /// §8.3
    pub const REMOTE_SFENCE_VMA_ASID: usize = 2;
    /// §8.4
    pub const REMOTE_HFENCE_GVMA_VMID: usize = 3;
    /// §8.5
    pub const REMOTE_HFENCE_GVMA: usize = 4;
    /// §8.6
    pub const REMOTE_HFENCE_VVMA_ASID: usize = 5;
    /// §8.7
    pub const REMOTE_HFENCE_VVMA: usize = 6;
}
