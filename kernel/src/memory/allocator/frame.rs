//! 页帧分配器

extern crate maturin_page_frame;

use crate::constants::{PAGE_SIZE, PHYS_MEMORY_OFFSET};
use crate::memory::get_phys_memory_regions;

pub struct PageFrameConfig;
impl maturin_page_frame::PageFrameConfig for PageFrameConfig {
    /// 物理地址转页帧编号
    fn phys_addr_to_frame_idx(addr: usize) -> usize {
        (addr - PHYS_MEMORY_OFFSET) / PAGE_SIZE
    }

    /// 页帧编号转物理地址
    fn frame_idx_to_phys_addr(idx: usize) -> usize {
        idx * PAGE_SIZE + PHYS_MEMORY_OFFSET
    }
}

pub type Frame = maturin_page_frame::Frame<PageFrameConfig>;

pub fn init() {
    unsafe { Frame::init(get_phys_memory_regions()) };
}
