use std::collections::BTreeMap;
//use alloc::collections::BTreeMap;

#[allow(dead_code)]
#[allow(non_snake_case)]
mod pteflags;
pub use pteflags::*;
mod segment;
pub use segment::Segment;
mod types;
pub use types::*;

pub struct RangeActionMap<SegmentType: Segment> {
    segments: BTreeMap<CmpType, RangeArea<SegmentType>>,
}

pub struct RangeArea<SegmentType: Segment> {
    start: CmpType,
    end: CmpType,
    pub segment: SegmentType,
}

impl<SegmentType: Segment> RangeArea<SegmentType> {
    #[inline]
    pub fn contains(&self, pos: CmpType) -> bool {
        self.start <= pos && pos < self.end
    }
}

impl<SegmentType: Segment> RangeActionMap<SegmentType> {
    /// 创建一个空的区间树
    pub fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
        }
    }
    /// 插入一段区间，不检查
    pub fn insert_raw(&mut self, start: CmpType, end: CmpType, segment: SegmentType) {
        self.segments.insert(start, RangeArea {
            start,
            end,
            segment,
        });
    }
    /// 查询某个地址是否在一个区间内，如是则返回区间引用，否则返回 None
    pub fn find<'a>(&'a mut self, pos: CmpType) -> Option<&'a SegmentType> {
        if let Some((_, area)) = self.segments.range(..=pos).last() {
            if area.contains(pos) {
                return Some(&area.segment);
            }
        }
        None
    }
}
