//! 将当前的 VmArea 去掉给定的区间后的差集
//!
//! 结果可能是原有 VmArea 被缩短，也可能是分成两段，也可能不变

use super::{RangeArea, Segment};

/// 当前 VmArea 和另一个给定的需要 unmap 的区间的相交关系
pub enum DiffSet<SegmentType: Segment> {
    /// 该区间和给定区间没有相交
    Unchanged,
    /// 当前区间完全被给定区间覆盖，因此应该删除
    Removed,
    /// 当前区间有一边和给定区间相交，为了让出地址空间需要缩短
    Shrinked,
    /// 当前区间覆盖了给定区间，为了让出地址空间需要分裂。返回分出的右半边
    Splitted(RangeArea<SegmentType>),
}

/// 当前 VmArea 和另一个给定的需要 mprotect 的区间的相交关系
pub enum CutSet<SegmentType: Segment> {
    /// 该区间和给定区间没有相交
    Unchanged,
    /// 该区间被给定区间覆盖，因此已整体修改权限
    WholeModified,
    /// 左边相交，已修改左半段的权限。返回分裂出的右半边
    ModifiedLeft(RangeArea<SegmentType>),
    /// 右边相交，已修改右半段的权限。返回分裂出的右半边
    ModifiedRight(RangeArea<SegmentType>),
    /// 当前区间覆盖了给定区间，已修改中间段的权限。返回分裂出的中间和右半边
    ModifiedMiddle(RangeArea<SegmentType>, RangeArea<SegmentType>),
}
