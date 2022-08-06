//! 用户地址空间中的指针、数组、数据等
//!

mod user_data;
mod user_ptr;

use super::MemorySet;

pub use user_ptr::{UserPtr, UserPtrUnchecked};
