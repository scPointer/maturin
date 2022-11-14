# RangeAction Map

一个区间树结构，用于提供 mmap / munmap / mprotect 时对区间的操作。

## 使用

本项目有 `std` 环境和 `no_std` 两个选项。

如需在 `std` 环境使用，直接在 `Cargo.toml` 引入即可；
如需在内核中使用，则需要选择：
```rust
range-action-map = { path = "https://github.com/scPointer/maturin/tree/memory-area-mod/range-action-map", default-features = false }
```

## 测试

本项目来自 `https://github.com/scPointer/maturin`。

其中 crate 源码在 `https://github.com/scPointer/maturin/tree/master/range-action-map`，
对这个 crate 本身的单元测试在  `https://github.com/scPointer/maturin/tree/master/range-action-map-test`。

单元测试本身只包含对数据结构本身的测试，不涉及页表和内存分配。实际在内存中的应用见下

## 应用

主要在 `https://github.com/scPointer/maturin/kernel` 模块中，
**下面的路径都以这个模块为当前路径做描述**

- 在 `src/memory/areas/mod.rs` 中，描述了内核里的 `VmArea` 如何实现这个模块的 `trait Segment`
- 在 `src/memory/vmm.rs` 中，描述了内核里的 `MemorySet` 如何使用这个模块的 `RangeActionMap`
- - 以及如何使用 `VmArea`
- 在 `Cargo.toml` 中，描述了如何引入本模块

## 对外提供接口

RangeActionMap 主要提供以下操作：
- `unmap(start, end)`：拆分/缩短/删除部分现有区间，空出`[start, end)` 这一段。
- `mprotect(start, end)`：修改所有区间与 `[start,end)` 相交的部分的属性。没有被 `[start, end)` 覆盖的区间会被拆分。
- `mmap_fixed(start, end)`：`unmap(start, end)`，并插入一个(用户给定的)新区间在`[start, end)`。
- `mmap_anywhere(hint, len)`：不修改任何区间，寻找一个长为 len 且左端点不小于 `hint` 的空位，并插入一个(用户给定的)新区间，返回插入位置的左端点。

还提供以下接口：
- `find(pos: usize)`：查询一个点是否在某个区间在，如果在，返回它的引用。
- `.iter()` `.iter_mut()`：迭代器支持。
- `impl Debug`：可以用 Debug 属性输出所有区间信息(需要用户提供的底层 `SegmentType` 实现 `Debug`)。

创建：
- `new(args: ArgsType)`


## 需要用户提供的接口和约定

`RangeActionMap` 需要一个实现 `trait Segment` 的类型，至少实现删除、拆分、修改三个操作：
- `remove()`：删除这段区间
- `split(pos)`：从`pos`位置把当前区间拆成两段区间(pos 参数为全局的绝对位置而非区间内位置)
- `modify(new_flags)`：修改区间的属性

一些约定：
- 删除区间时需要用户底层结构完成返还页帧、修改页表等操作，但不需要 `Drop` 结构本身
- 其中每个区间有一个 usize 大小的可修改的属性，在用于内存管理时，它一般是 PTEFlags
- - (尽管这个结构只需要u8，但我们希望这个属性至少可以放下一个 raw_pointer 以便扩展其他用途)。
- **此外，`RangeActionMap`创建时要求传入一个 `ArgsType`，它实际上是一个 usize。这个值会在每次操作时传递给底层区间**
