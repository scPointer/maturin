//! # RangeAction Map
//! 
//! 一个区间树结构，用于提供 mmap / munmap / mprotect 时对区间的操作。
//! 
//! ## 使用
//! 
//! 本项目有 `std` 环境和 `no_std` 两个选项。
//! 
//! 如需在 `std` 环境使用，直接在 `Cargo.toml` 引入即可；
//! 如需在内核中使用，则需要选择：
//! ```
//! range-action-map = { path = "https://github.com/scPointer/maturin/tree/memory-area-mod/range-action-map", default-features = false }
//! ```
//! 
//! ## 测试
//! 
//! 本项目来自 `https://github.com/scPointer/maturin`。
//! 
//! 其中 crate 源码在 `https://github.com/scPointer/maturin/tree/master/range-action-map`，
//! 对这个 crate 本身的单元测试在  `https://github.com/scPointer/maturin/tree/master/range-action-map-test`。
//! 
//! 单元测试本身只包含对数据结构本身的测试，不涉及页表和内存分配。实际在内存中的应用见下
//! 
//! ## 应用
//! 
//! 主要在 `https://github.com/scPointer/maturin/kernel` 模块中，
//! **下面的路径都以这个模块为当前路径做描述**
//! 
//! - 在 `src/memory/areas/mod.rs` 中，描述了内核里的 `VmArea` 如何实现这个模块的 `trait Segment`
//! - 在 `src/memory/vmm.rs` 中，描述了内核里的 `MemorySet` 如何使用这个模块的 `RangeActionMap`
//! - - 以及如何使用 `VmArea`
//! - 在 `Cargo.toml` 中，描述了如何引入本模块
//! 
//! ## 对外提供接口
//! 
//! RangeActionMap 主要提供以下操作：
//! - `unmap(start, end)`：拆分/缩短/删除部分现有区间，空出`[start, end)` 这一段。
//! - `mprotect(start, end)`：修改所有区间与 `[start,end)` 相交的部分的属性。没有被 `[start, end)` 覆盖的区间会被拆分。
//! - `mmap_fixed(start, end)`：`unmap(start, end)`，并插入一个(用户给定的)新区间在`[start, end)`。
//! - `mmap_anywhere(hint, len)`：不修改任何区间，寻找一个长为 len 且左端点不小于 `hint` 的空位，并插入一个(用户给定的)新区间，返回插入位置的左端点。
//! 
//! 还提供以下接口：
//! - `find(pos: usize)`：查询一个点是否在某个区间在，如果在，返回它的引用。
//! - `.iter()` `.iter_mut()`：迭代器支持。
//! - `impl Debug`：可以用 Debug 属性输出所有区间信息(需要用户提供的底层 `SegmentType` 实现 `Debug`)。
//! 
//! 创建：
//! - `new(args: ArgsType)`
//! 
//! 
//! ## 需要用户提供的接口和约定
//! 
//! `RangeActionMap` 需要一个实现 `trait Segment` 的类型，至少实现删除、拆分、修改三个操作：
//! - `remove()`：删除这段区间
//! - `split(pos)`：从`pos`位置把当前区间拆成两段区间(pos 参数为全局的绝对位置而非区间内位置)
//! - `modify(new_flags)`：修改区间的属性
//! 
//! 一些约定：
//! - 删除区间时需要用户底层结构完成返还页帧、修改页表等操作，但不需要 `Drop` 结构本身
//! - 其中每个区间有一个 usize 大小的可修改的属性，在用于内存管理时，它一般是 PTEFlags
//! - - (尽管这个结构只需要u8，但我们希望这个属性至少可以放下一个 raw_pointer 以便扩展其他用途)。
//! - **此外，`RangeActionMap`创建时要求传入一个 `ArgsType`，它实际上是一个 usize。这个值会在每次操作时传递给底层区间**
//!
 
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(btree_drain_filter)]

#[cfg(feature = "std")]
mod external {
    pub use std::collections::btree_map::{Iter, IterMut};
    pub use std::{collections::BTreeMap, fmt::Debug, iter::Iterator, vec::Vec};
}
#[cfg(not(feature = "std"))]
mod external {
    extern crate alloc;
    pub use alloc::collections::btree_map::{Iter, IterMut};
    pub use alloc::collections::BTreeMap;
    pub use alloc::vec::Vec;
    pub use core::fmt::Debug;
    pub use core::iter::Iterator;
}

use external::*;

mod segment;
pub use segment::Segment;
mod set;
pub use set::{CutSet, DiffSet};
mod range_area;
pub use range_area::RangeArea;
mod defs;
pub use defs::{ArgsType, IdentType, LOWER_LIMIT, UPPER_LIMIT};

/// 一个数据结构，维护互不相交的左闭右开区间。
/// 其中每个区间有一个 usize 大小的可修改的属性，在用于内存管理时，它一般是 PTEFlags
/// (尽管这个结构只需要u8，但我们希望这个属性至少可以放下一个 raw_pointer 以便扩展其他用途)。
/// 
/// RangeActionMap 主要提供以下操作：
/// - `unmap(start, end)`：拆分/缩短/删除部分现有区间，空出`[start, end)` 这一段。
/// - `mprotect(start, end)`：修改所有区间与 `[start,end)` 相交的部分的属性。没有被 `[start, end)` 覆盖的区间会被拆分。
/// - `mmap_fixed(start, end)`：`unmap(start, end)`，并插入一个(用户给定的)新区间在`[start, end)`。
/// - `mmap_anywhere(hint, len)`：不修改任何区间，寻找一个长为 len 且左端点不小于 `hint` 的空位，并插入一个(用户给定的)新区间，返回插入位置的左端点。
/// 
/// 还提供以下接口：
/// - `find(pos: usize)`：查询一个点是否在某个区间在，如果在，返回它的引用。
/// - `.iter()` `.iter_mut()`：迭代器支持。
/// - `impl Debug`：可以用 Debug 属性输出所有区间信息(需要用户提供的底层 `SegmentType` 实现 `Debug`)。
/// 
/// `RangeActionMap` 需要一个实现 `trait Segment` 的类型，至少实现删除、拆分、修改三个操作：
/// - `remove()`：删除这段区间
/// - `split(pos)`：从`pos`位置把当前区间拆成两段区间(pos 参数为全局的绝对位置而非区间内位置)
/// - `modify(new_flags)`：修改区间的属性
/// 
/// **此外，`RangeActionMap`创建时要求传入一个 `ArgsType`，它实际上是一个 usize。这个值会在每次操作时传递给底层区间**
/// 
/// # Example
/// 
/// ```
/// # use range_action_map::{RangeActionMap, Segment, IdentType, ArgsType};
/// /// 定义一个区间结构，内部只保存左右端点
/// struct Seg(usize, usize);
/// let mut ram = RangeActionMap::<Seg>::new(ArgsType::default());
/// /// 分配一段区间，注意区间是**左闭右开**的
/// ram.mmap_fixed(0x3000, 0x7000, || { Seg(0x3000, 0x7000) });
/// assert!(ram.find(0x2111).is_none());
/// assert!(ram.find(0x3000).is_some());
/// assert!(ram.find(0x5678).is_some());
/// assert!(ram.find(0x7000).is_none());
/// 
/// /// 实现 Segment
/// impl Segment for Seg {
///     fn remove(&mut self, args: ArgsType) {}
///     fn modify(&mut self, new_flag: IdentType, args: ArgsType) {}
///     fn split(&mut self, pos: usize, args: ArgsType) -> Self {
///         let right_end = self.1;
///         self.1 = pos;
///         Self(pos, right_end)
///     }
/// }
/// ```
/// 
pub struct RangeActionMap<SegmentType: Segment> {
    pub segments: BTreeMap<usize, RangeArea<SegmentType>>,
    args: ArgsType,
}

impl<SegmentType: Segment> RangeActionMap<SegmentType> {
    /// 创建一个空的区间树。
    /// 
    /// 传入的 `args` 会在每次操作时传递给底层的区间。
    /// (如果用于内存管理，推荐传入页表位置)
    /// 
    /// 此外，也可在 `./defs.rs` 修改 `ArgsType` 的定义，以传递不同的参数
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
    pub fn find<'a>(&'a self, pos: usize) -> Option<&'a SegmentType> {
        if let Some((_, area)) = self.segments.range(..=pos).last() {
            if area.contains(pos) {
                return Some(&area.segment);
            }
        }
        None
    }
    /// 通过迭代器访问每个区间的引用
    pub fn iter<'a>(&'a self) -> RangeActionMapIter<'a, SegmentType> {
        RangeActionMapIter {
            map_iter: self.segments.iter(),
        }
    }
    /// 通过迭代器访问每个区间的可变引用
    pub fn iter_mut<'a>(&'a mut self) -> RangeActionMapIterMut<'a, SegmentType> {
        RangeActionMapIterMut {
            map_iter: self.segments.iter_mut(),
        }
    }
    /// 插入一段长度为 len 的区间，且区间左端点位置不小于 hint。
    /// 
    /// - 如找到这样的区间，则会执行 `f(start: usize)` 获取区间实例，
    /// 然后返回 Some(start) 表示区间左端点；
    /// - 否则，**不会执行 `f` **，并返回 None
    /// 
    /// # Example
    /// 
    /// ```
    /// # use range_action_map::{RangeActionMap, Segment, IdentType, ArgsType};
    /// /// 定义一个区间结构，内部只保存左右端点
    /// struct Seg(usize, usize);
    /// /// 实现 Segment
    /// impl Segment for Seg {
    ///     fn remove(&mut self, args: ArgsType) {}
    ///     fn modify(&mut self, new_flag: IdentType, args: ArgsType) {}
    ///     fn split(&mut self, pos: usize, args: ArgsType) -> Self {
    ///         let right_end = self.1;
    ///         self.1 = pos;
    ///         Self(pos, right_end)
    ///     }
    /// }
    /// let mut map: RangeActionMap<Seg> = RangeActionMap::new(0);
    /// /// 申请一个长为 10 的区间，要求左端点不小于 123，获得[123,133)
    /// assert_eq!(Some(123), map.mmap_anywhere(123, 10, |start| Seg(start, start+10)));
    /// /// 申请一个长为 10 的区间，要求左端点不小于 140，获得[140,150)
    /// assert_eq!(Some(140), map.mmap_anywhere(140, 10, |start| Seg(start, start+10)));
    /// /// 申请一个长为 10 的区间，要求左端点不小于 120，获得[150, 160)。
    /// /// 这是因为[123,133)和[140,150)已有区间，第一个满足要求的空区间只能从 150 开始
    /// assert_eq!(Some(150), map.mmap_anywhere(120, 10, |start| Seg(start, start+10)));
    /// ```
    pub fn mmap_anywhere(
        &mut self,
        hint: usize,
        len: usize,
        f: impl FnOnce(usize) -> SegmentType,
    ) -> Option<usize> {
        self.find_free_area(hint, len).map(|start| {
            self.insert_raw(start, start + len, f(start));
            start
        })
    }

    /// 尝试插入一个区间。如插入成功，返回插入后的起始地址
    ///
    /// - 如区间在 `[LOWER_LIMIT, UPPER_LIMIT]` 范围内，则会：
    /// - - unmap(start, end)，
    /// - - 然后执行 `f(start: usize)` ，
    /// - - 然后返回 Some(start) 表示区间左端点；
    /// - 否则，**不会执行 `f` **，并返回 None
    pub fn mmap_fixed(
        &mut self,
        start: usize,
        end: usize,
        f: impl FnOnce() -> SegmentType,
    ) -> Option<usize> {
        if start < LOWER_LIMIT || end > UPPER_LIMIT {
            return None;
        }
        // 需要 unmap 掉原本相交的区间
        self.unmap(start, end);
        self.insert_raw(start, end, f());
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
    pub fn mprotect(&mut self, start: usize, end: usize, new_flags: IdentType) {
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
                    self.segments.insert(area.start, area);
                }
                CutSet::ModifiedLeft(right) | CutSet::ModifiedRight(right) => {
                    // 在 split_and_modify_if_overlap 内部已经处理过了修改 flags 的部分
                    // 所以如果有半边相交，可以直接把切出的区间塞回 self.areas
                    self.segments.insert(area.start, area);
                    self.segments.insert(right.start, right);
                }
                CutSet::ModifiedMiddle(mid, right) => {
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
            last_seg_end = last_seg_end.max(seg.end);
        }
        if last_seg_end + len <= UPPER_LIMIT {
            Some(last_seg_end)
        } else {
            None
        }
    }
}

impl<SegmentType: Segment + Debug> Debug for RangeActionMap<SegmentType> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RangeActionMap")
            .field("segments", &self.segments.values())
            .finish()
    }
}

pub struct RangeActionMapIter<'a, SegmentType: Segment> {
    map_iter: Iter<'a, usize, RangeArea<SegmentType>>,
}

impl<'a, SegmentType: Segment> Iterator for RangeActionMapIter<'a, SegmentType> {
    type Item = &'a SegmentType;
    fn next(&mut self) -> Option<Self::Item> {
        self.map_iter.next().map(|(_, area)| &area.segment)
    }
}

pub struct RangeActionMapIterMut<'a, SegmentType: Segment> {
    map_iter: IterMut<'a, usize, RangeArea<SegmentType>>,
}

impl<'a, SegmentType: Segment> Iterator for RangeActionMapIterMut<'a, SegmentType> {
    type Item = &'a mut SegmentType;
    fn next(&mut self) -> Option<Self::Item> {
        self.map_iter.next().map(|(_, area)| &mut area.segment)
    }
}
