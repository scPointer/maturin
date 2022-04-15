mod heap;
mod frame;

pub use frame::Frame;

use super::{
    get_phys_memory_regions,
    phys_to_virt,
};
use super::{PhysAddr, PAGE_SIZE, PHYS_MEMORY_OFFSET};

/// 初始化堆分配器和页帧分配器。需由其中一个核调用且仅调用一次
pub fn allocator_init() {
    heap::init();
    println!("heap init end");
    frame::init();
    println!("frame init end");
}
