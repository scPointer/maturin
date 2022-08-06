//! Chapter 4. Base Extension (EID #0x10)

pub const EID_BASE: usize = 0x10;

pub use fid::*;

pub const UNAVAILABLE_EXTENSION: usize = 0;

/// §4.1
#[repr(transparent)]
pub struct SbiSpecVersion(pub usize);

impl SbiSpecVersion {
    #[inline]
    pub const fn major(&self) -> usize {
        (self.0 >> 24) & ((1 << 7) - 1)
    }

    #[inline]
    pub const fn minor(&self) -> usize {
        self.0 & ((1 << 24) - 1)
    }
}

use core::fmt::{Display, Formatter, Result};

impl Display for SbiSpecVersion {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}.{}", self.major(), self.minor())
    }
}

/// §4.8
mod fid {
    /// §4.1
    pub const GET_SBI_SPEC_VERSION: usize = 0x0;
    /// §4.2
    pub const GET_SBI_IMPL_ID: usize = 0x1;
    /// §4.3
    pub const GET_SBI_IMPL_VERSION: usize = 0x2;
    /// §4.4
    pub const PROBE_EXTENSION: usize = 0x3;
    /// §4.5
    pub const GET_MVENDORID: usize = 0x4;
    /// §4.6
    pub const GET_MARCHID: usize = 0x5;
    /// §4.7
    pub const GET_MIMPID: usize = 0x6;
}

/// §4.9
pub mod impl_id {
    pub const BBL: usize = 0;
    pub const OPEN_SBI: usize = 1;
    pub const XVISOR: usize = 2;
    pub const KVM: usize = 3;
    pub const RUST_SBI: usize = 4;
    pub const DIOSIX: usize = 5;
    pub const COFFER: usize = 6;
}
