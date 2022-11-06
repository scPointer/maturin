//! 一段数据结构内维护的区间，相比用户给出的区间，还需要额外存区间端点

use super::{ArgsType, CutSet, DiffSet, PTEFlags, Segment};
pub struct RangeArea<SegmentType: Segment> {
    pub start: usize,
    pub end: usize,
    pub segment: SegmentType,
}

impl<SegmentType: Segment> RangeArea<SegmentType> {
    #[inline]
    /// 当前区间是否包含 pos 这个点
    pub fn contains(&self, pos: usize) -> bool {
        self.start <= pos && pos < self.end
    }
    /// 尝试空出[start, end)区间。即删除当前区间中和[start, end)相交的部分。
    ///
    /// **注意，这个函数在内部已经 unmap 了对应的区间中的映射，调用后不需要再手动 unmap**。
    /// 这个函数默认参数中的 start 和 end 是按页对齐的
    pub fn shrink_or_split_if_overlap(
        &mut self,
        start: usize,
        end: usize,
        args: ArgsType,
    ) -> DiffSet<SegmentType> {
        if end <= self.start || self.end <= start {
            // 不相交
            DiffSet::Unchanged
        } else if start <= self.start && self.end <= end {
            // 被包含
            self.segment.remove(args);
            DiffSet::Removed
        } else if self.start < start && end < self.end {
            // 需要分割
            let old_end = self.end;
            self.end = start;
            DiffSet::Splitted(Self {
                start: end,
                end: old_end,
                segment: self.segment.split_and_remove_middle(start, end, args),
            })
        } else if end < self.end {
            // 需要删除前半段
            self.segment.shrink_to_right(end, args);
            self.start = end;
            DiffSet::Shrinked
        } else {
            // 删除后半段
            assert_eq!(self.start < start, true); // 最后一种情况一定是后半段重叠
            self.segment.shrink_to_left(start, args);
            self.end = start;
            DiffSet::Shrinked
        }
    }
    /// 尝试修改与 [start, end) 区间相交的部分的权限。
    /// 如果这一修改导致区间分裂，则分别返回分出的每个区间。
    pub fn split_and_modify_if_overlap(
        &mut self,
        start: usize,
        end: usize,
        new_flag: PTEFlags,
        args: ArgsType,
    ) -> CutSet<SegmentType> {
        if end <= self.start || self.end <= start {
            // 不相交
            CutSet::Unchanged
        } else if start <= self.start && self.end <= end {
            // 被包含
            self.segment.modify(new_flag, args);
            CutSet::WholeModified
        } else if self.start < start && end < self.end {
            // 包含区间，需要分割三段
            let pos_left = start; // 第一个裁剪点
            let pos_right = end; // 第二个裁剪点
            let (seg_middle, seg_right) = self
                .segment
                .modify_middle(pos_left, pos_right, new_flag, args);
            let old_end = self.end;
            self.end = start;
            CutSet::ModifiedMiddle(
                Self {
                    start: start,
                    end: end,
                    segment: seg_middle,
                },
                Self {
                    start: end,
                    end: old_end,
                    segment: seg_right,
                },
            )
        } else if end < self.end {
            // 前半段相交
            let old_end = self.end;
            self.end = start;
            // 注意返回的是右半段
            CutSet::ModifiedLeft(Self {
                start: end,
                end: old_end,
                segment: self.segment.modify_left(end, new_flag, args),
            })
        } else {
            // 后半段相交
            assert_eq!(self.start < start, true); // 最后一种情况一定是后半段重叠
            let old_end = self.end;
            self.end = start;
            CutSet::ModifiedRight(Self {
                start: end,
                end: old_end,
                segment: self.segment.modify_right(end, new_flag, args),
            })
        }
    }

    /// 当前区间与 [start, end) 是否相交
    pub fn is_overlap_with(&self, start: usize, end: usize) -> bool {
        !(self.end <= start || self.start >= end)
    }
}
