//! 将当前的 VmArea 去掉给定的区间后的差集
//!
//! 结果可能是原有 VmArea 被缩短，也可能是分成两段，也可能不变

use super::VmArea;

/// 当前 VmArea 和另一个给定的需要 unmap 的区间的相交关系
pub enum DiffSet {
    /// 该区间和给定区间没有相交
    Unchanged,
    /// 当前区间完全被给定区间覆盖，因此应该删除
    Removed,
    /// 当前区间有一边和给定区间相交，为了让出地址空间需要缩短
    Shrinked,
    /// 当前区间覆盖了给定区间，为了让出地址空间需要分裂
    Splitted(VmArea, VmArea),
}

/// 当前 VmArea 和另一个给定的需要 mprotect 的区间的相交关系
/// 这里仅考虑区间已经相交的情况，如果不确认是否相交，需要先调用 VmArea::is_overlap_with
pub enum CutSet {
    /// 该区间和给定区间没有相交
    Unchanged,
    /// 该区间被给定区间覆盖，因此已整体修改权限
    WholeModified,
    /// 左边相交，已修改左半段的权限
    ModifiedLeft(VmArea, VmArea),
    /// 右边相交，已修改右半段的权限
    ModifiedRight(VmArea, VmArea),
    /// 当前区间覆盖了给定区间，已修改中间段的权限
    ModifiedMiddle(VmArea, VmArea, VmArea),
}
