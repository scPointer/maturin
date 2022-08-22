//! 各种分配器
//!
//! - 使用 buddy_system_allocator::LockedHeap 作为堆分配器；
//! - 使用 bitmap_allocator 作为其他编号的分配器，这个类型里的实现是用 bitset 做 radix tree

//#![deny(missing_docs)]

mod fd;
mod frame;
mod heap;
mod tid;

use super::{get_phys_memory_regions, phys_to_virt};
use super::{PhysAddr, PAGE_SIZE, PHYS_MEMORY_OFFSET};

pub use fd::FdAllocator;
pub use frame::Frame;
pub use tid::Tid;

/// 初始化堆分配器、页帧分配器和 TID 分配器。需由其中一个核调用且仅调用一次
pub fn allocator_init() {
    // println 中调用的 STDOUT 有 Mutex 锁，需要在堆上分配
    // 所以在 heap::init() 前请不要输出任何语句
    heap::init();
    info!("heap allocator inited.");
    frame::init();
    info!("frame allocator inited.");
    tid::init();
    info!("tid allocator inited.");
}
