mod heap;
mod frame;
mod pid;

use super::{
    get_phys_memory_regions,
    phys_to_virt,
};
use super::{PhysAddr, PAGE_SIZE, PHYS_MEMORY_OFFSET};

pub use frame::Frame;
pub use pid::Pid;

/// 初始化堆分配器、页帧分配器和 PID 分配器。需由其中一个核调用且仅调用一次
pub fn allocator_init() {
    // println 中调用的 STDOUT 有 Mutex 锁，需要在堆上分配
    // 所以在 heap::init() 前请不要输出任何语句
    heap::init();
    println!("heap allocator inited.");
    frame::init();
    println!("frame allocator inited.");
    pid::init();
    println!("pid allocator inited.");
}
