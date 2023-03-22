//! 错误类型

#![allow(dead_code)]

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod external {
    pub use std::result::Result;
}

#[cfg(not(feature = "std"))]
mod external {
    pub use core::result::Result;
}

pub use external::*;

#[cfg(feature = "numeric_enum")]
mod numeric_enum;
#[cfg(feature = "numeric_enum")]
pub use numeric_enum::OSError;

#[cfg(feature = "enum")]
mod enums;
#[cfg(feature = "enum")]
pub use enums::OSError;

#[cfg(feature = "numeric")]
mod numeric;
#[cfg(feature = "numeric")]
pub use numeric::OSError;

pub type OSResult<T = ()> = Result<T, OSError>;