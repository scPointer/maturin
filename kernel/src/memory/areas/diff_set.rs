//! 将当前的 VmArea 去掉给定的区间后的差集
//!
//! 结果可能是原有 VmArea 被缩短，也可能是分成两段，也可能不变

use super::VmArea;

/// 给定的 VmArea 去掉给定的区间后的差集
pub enum DiffSet {
    /// 该区间和给定区间没有相交
    Unchanged,
    /// 该区间完全被给定区间覆盖，因此应该删除
    Removed,
    /// 该区间为了让出地址空间需要缩短
    Shrinked,
    /// 该区间为了让出地址空间需要分裂
    Splitted(VmArea, VmArea),
}
