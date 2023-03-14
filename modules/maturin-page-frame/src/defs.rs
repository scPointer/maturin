//! 单独运行/测试当前模块时需要的定义。
//!

/// 页表中每页的大小
const PAGE_SIZE: usize = 0x1000; // 4 KB

/// 内核中虚拟地址相对于物理地址的偏移
///
/// 即：内核可以通过访问 (x + PHYS_VIRT_OFFSET) 拿到物理地址 x 处的值
const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;

pub trait PageFrameConfig {
    /// 页面大小
    fn get_page_size() -> usize {
        PAGE_SIZE
    }
    /// 物理地址转虚拟地址(仅限内核偏移映射)
    fn phys_addr_to_virt_addr(paddr: usize) -> usize {
        paddr + PHYS_VIRT_OFFSET
    }

    /// 物理地址转页帧编号
    fn phys_addr_to_frame_idx(addr: usize) -> usize {
        addr / PAGE_SIZE
    }

    /// 页帧编号转物理地址
    fn frame_idx_to_phys_addr(idx: usize) -> usize {
        idx * PAGE_SIZE
    }
}
