//! 模块需要用到的定义
//!

/// 页表中每页的大小
const PAGE_SIZE: usize = 0x1000; // 4 KB

/// 内核中虚拟地址相对于物理地址的偏移
///
/// 即：内核可以通过访问 (x + PHYS_VIRT_OFFSET) 拿到物理地址 x 处的值
const PHYS_VIRT_OFFSET: usize = 0xFFFF_FFFF_0000_0000;

pub trait MemoryAreaConfig {
    /// 页面大小
    fn get_page_size() -> usize {
        PAGE_SIZE
    }
    /// 物理地址转虚拟地址(仅限内核偏移映射)
    fn phys_addr_to_virt_addr(paddr: usize) -> usize {
        paddr + PHYS_VIRT_OFFSET
    }
    /// 页首地址
    pub fn align_down(addr: usize) -> usize {
        addr & !(Self::get_page_size() - 1)
    }

    /// 下一页页首地址
    pub fn align_up(addr: usize) -> usize {
        (addr + Self::get_page_size() - 1) & !(Self::get_page_size() - 1)
    }

    /// 从地址获取页号
    pub fn addr_to_page_id(addr: usize) -> usize {
        addr / Self::get_page_size()
    }

    /// 需要多少页来存放 size Byte 的数据
    pub fn page_count(size: usize) -> usize {
        Self::align_up(size) / Self::get_page_size()
    }
}
