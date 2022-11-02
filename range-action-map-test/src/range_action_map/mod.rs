use std::{collections::BTreeMap, env::Args};
//use alloc::collections::BTreeMap;

#[allow(dead_code)]
#[allow(non_snake_case)]
mod pteflags;
pub use pteflags::*;
mod segment;
pub use segment::Segment;
mod set;
pub use set::{CutSet, DiffSet};
mod defs;
mod outer;
pub use defs::*;
pub use outer::*;

use crate::Seg;

pub struct RangeActionMap<SegmentType: Segment> {
    segments: BTreeMap<usize, RangeArea<SegmentType>>,
    args: ArgsType,
}

pub struct RangeArea<SegmentType: Segment> {
    start: usize,
    end: usize,
    pub segment: SegmentType,
}

impl<SegmentType: Segment> RangeArea<SegmentType> {
    #[inline]
    /// 当前区间是否包含 pos 这个点
    pub fn contains(&self, pos: usize) -> bool {
        self.start <= pos && pos < self.end
    }
    /*
    /// 已知区间 [start, end)，其中 self.start < start，询问当前区间与该区间关系
    pub fn relation_to_right(&self, start: usize, end: usize) -> SegRelation {
        if self.end <= start {
            SegRelation::Disjoint
        } else if self.end < end {
            SegRelation::Intersect
        } else {
            SegRelation::Contain
        }
    }
    /// 已知区间 [start, end)，其中 self.start >= start，询问当前区间与该区间关系
    pub fn relation_to_left(&self, _start: usize, end: usize) -> SegRelation {
        if self.start >= end {
            SegRelation::Disjoint
        } else if self.end > end {
            SegRelation::Intersect
        } else {
            SegRelation::BeContained
        }
    }
     */
    /// 当前区间与 [start, end) 是否相交
    pub fn is_overlap_with(&self, start: usize, end: usize) -> bool {
        !(self.end <= start || self.start >= end)
    }
}

impl<SegmentType: Segment> RangeActionMap<SegmentType> {
    /// 创建一个空的区间树
    pub fn new(args: ArgsType) -> Self {
        Self {
            segments: BTreeMap::new(),
            args,
        }
    }
    /// 插入一段区间，不检查
    pub unsafe fn insert_raw(&mut self, start: usize, end: usize, segment: SegmentType) {
        self.segments.insert(
            start,
            RangeArea {
                start,
                end,
                segment,
            },
        );
    }
    /// 查询某个地址是否在一个区间内，如是则返回区间引用，否则返回 None
    pub fn find<'a>(&'a mut self, pos: usize) -> Option<&'a SegmentType> {
        if let Some((_, area)) = self.segments.range(..=pos).last() {
            if area.contains(pos) {
                return Some(&area.segment);
            }
        }
        None
    }
    /// 映射一段长度为 len 的区间，且区间左端点位置不小于 hint。
    ///
    /// 如找到这样的区间，则会执行 `f(&mut segment, start)` 以便在其中操作页表，
    /// 然后返回 Some(start) 表示区间左端点；
    /// 否则，返回 None
    pub unsafe fn mmap_anywhere(
        &mut self,
        hint: usize,
        len: usize,
        mut segment: SegmentType,
        f: impl Fn(&mut SegmentType, usize) -> (),
    ) -> Option<usize> {
        self.find_free_area(hint, len).map(|start| {
            f(&mut segment, start);
            self.insert_raw(start, start + len, segment);
            start
        })
    }
    /// 删除映射，空出 [start, end) 这段区间。
    ///
    /// 这可能导致一些区间被缩短或拆分
    pub fn unmap(&mut self, start: usize, end: usize) {
        let areas_to_be_modified: Vec<RangeArea<SegmentType>> = self
            .segments
            .drain_filter(|_, area| area.is_overlap_with(start, end))
            .map(|(_, v)| v)
            .collect();
    }
    pub fn find_free_area(&self, hint: usize, len: usize) -> Option<usize> {
        // 上一段区间的末尾
        let mut last_seg_end = hint.max(LOWER_LIMIT);
        for (start, seg) in self.segments.iter() {
            // 现在检查从上一段末尾到这一段开头能不能塞下一个长为 len 的段
            if last_seg_end + len <= *start {
                return Some(last_seg_end);
            }
            last_seg_end = seg.end;
        }
        None
    }
}
