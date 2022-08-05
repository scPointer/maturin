//! 堆分配器
//!
//! 使用 buddy_system_allocator::LockedHeap ，堆大小在 constants 中定义

//#![deny(missing_docs)]

use buddy_system_allocator::LockedHeap;

use crate::constants::KERNEL_HEAP_SIZE;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::new();

/// Initialize the global heap alloactor.
pub fn init() {
    const MACHINE_ALIGN: usize = core::mem::size_of::<usize>();
    const HEAP_BLOCK: usize = KERNEL_HEAP_SIZE / MACHINE_ALIGN;
    static mut HEAP: [usize; HEAP_BLOCK] = [0; HEAP_BLOCK];
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, HEAP_BLOCK * MACHINE_ALIGN);
    };
}
