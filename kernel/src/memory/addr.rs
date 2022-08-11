//! 与地址映射相关的函数定义

//#![deny(missing_docs)]

use super::{PAGE_SIZE, PHYS_VIRT_OFFSET};
use core::mem::size_of;

pub type VirtAddr = usize;
pub type PhysAddr = usize;

/// 物理地址转虚拟地址(仅限内核偏移映射)
pub fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    paddr + PHYS_VIRT_OFFSET
}

/// 虚拟地址转物理地址(仅限内核偏移映射)
pub fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    vaddr - PHYS_VIRT_OFFSET
}

/// 页首地址
pub fn align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// 下一页页首地址
pub fn align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// 检查一个结构是否跨页
pub fn cross_page<T>(addr: usize) -> bool {
    (addr ^ (addr + size_of::<T>())) >= PAGE_SIZE
}

/// 需要多少页来存放 size Byte 的数据
pub fn page_count(size: usize) -> usize {
    align_up(size) / PAGE_SIZE
}

/// 从地址获取页号
pub fn addr_to_page_id(addr: usize) -> usize {
    addr / PAGE_SIZE
}

/// 从页号获取开头地址
pub fn page_id_to_addr(page_id: usize) -> usize {
    page_id * PAGE_SIZE
}

/// 地址转页内偏移
pub fn page_offset(addr: usize) -> usize {
    addr & (PAGE_SIZE - 1)
}

/// 虚拟地址所对应的Sv39的三级页表项，即第 \[38:30\],\[29:21\],\[20:12\] 位
pub fn pte_idx_of_virt_addr(vaddr: VirtAddr) -> (usize, usize, usize) {
    (
        (vaddr >> 30) & 0x1ff,
        (vaddr >> 21) & 0x1ff,
        (vaddr >> 12) & 0x1ff,
    )
}
