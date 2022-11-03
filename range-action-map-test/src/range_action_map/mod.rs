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
mod range_area;
pub use range_area::RangeArea;
mod defs;
pub use defs::*;
mod outer;
pub use outer::*;

use crate::Seg;

pub struct RangeActionMap<SegmentType: Segment> {
    segments: BTreeMap<usize, RangeArea<SegmentType>>,
    args: ArgsType,
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
    fn insert_raw(&mut self, start: usize, end: usize, segment: SegmentType) {
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
    pub fn mmap_anywhere(
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

    /// 尝试插入一段数据。如插入成功，返回插入后的起始地址
    ///
    /// 必须在 [start, end) 尝试插入。
    pub fn mmap_fixed(
        &mut self,
        start: usize,
        end: usize,
        mut segment: SegmentType,
        f: impl Fn(&mut SegmentType, usize) -> (),
    ) -> Option<usize> {
        if start < LOWER_LIMIT || end > UPPER_LIMIT {
            return None;
        }
        // 需要 unmap 掉原本相交的区间
        self.unmap(start, end);
        f(&mut segment, start);
        self.insert_raw(start, end, segment);
        Some(start)
    }
    /// 删除映射，空出 [start, end) 这段区间。
    ///
    /// 这可能导致一些区间被缩短或拆分
    pub fn unmap(&mut self, start: usize, end: usize) {
        // 注意，这里把相交的区间直接从 self.areas 里取出来了
        // 所以如果仅相交而不需要删除，就需要放回 self.areas
        let areas_to_be_modified: Vec<RangeArea<SegmentType>> = self
            .segments
            .drain_filter(|_, area| area.is_overlap_with(start, end))
            .map(|(_, v)| v)
            .collect();
        for mut area in areas_to_be_modified {
            match area.shrink_or_split_if_overlap(start, end, self.args) {
                DiffSet::Shrinked => {
                    println!("try shrink to {:x}, {:x}", area.start, area.end);
                    self.segments.insert(area.start, area);
                }
                DiffSet::Splitted(right) => {
                    self.segments.insert(area.start, area);
                    self.segments.insert(right.start, right);
                }
                _ => {} // 被删除或者未相交时，就不需要再管了
            }
        }
    }
    /// 调整所有和已知区间相交的区间，修改 [start, end) 段的权限。
    /// 它可以直接当作 mprotect 使用
    pub fn mprotect(&mut self, start: usize, end: usize, new_flags: PTEFlags) {
        // 注意，这里把相交的区间直接从 self.areas 里取出来了
        // 所以如果仅相交而不需要删除，就需要放回 self.areas
        let areas_to_be_modified: Vec<RangeArea<SegmentType>> = self
            .segments
            .drain_filter(|_, area| area.is_overlap_with(start, end))
            .map(|(_, v)| v)
            .collect();
        for mut area in areas_to_be_modified {
            match area.split_and_modify_if_overlap(start, end, new_flags, self.args) {
                CutSet::WholeModified => {
                    //info!("mprotect: modified {:x}, {:x}", area.start, area.end);
                    self.segments.insert(area.start, area);
                }
                CutSet::ModifiedLeft(right) | CutSet::ModifiedRight(right) => {
                    // 在 split_and_modify_if_overlap 内部已经处理过了修改 flags 的部分
                    // 所以如果有半边相交，可以直接把切出的区间塞回 self.areas
                    //info!("mprotect: cut and modified one-side {:x}, {:x}", area.start, area.end);
                    self.segments.insert(area.start, area);
                    self.segments.insert(right.start, right);
                }
                CutSet::ModifiedMiddle(mid, right) => {
                    //info!("mprotect: cut and modified middle {:x}, {:x}", area.start, area.end);
                    self.segments.insert(area.start, area);
                    self.segments.insert(mid.start, mid);
                    self.segments.insert(right.start, right);
                }
                _ => {} // 未相交时，就不需要再管了
            }
        }
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
