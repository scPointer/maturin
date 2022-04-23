//! Definition of phyical and virtual addresses.

use super::{PAGE_SIZE, PHYS_VIRT_OFFSET};

pub type VirtAddr = usize;
pub type PhysAddr = usize;

pub fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    paddr + PHYS_VIRT_OFFSET
}

pub fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    vaddr - PHYS_VIRT_OFFSET
}

pub fn align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

pub fn align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub fn is_aligned(addr: usize) -> bool {
    page_offset(addr) == 0
}

pub fn page_count(size: usize) -> usize {
    align_up(size) / PAGE_SIZE
}

pub fn page_offset(addr: usize) -> usize {
    addr & (PAGE_SIZE - 1)
}

pub fn page_id_to_addr(id: usize) -> usize {
    id * PAGE_SIZE
}

/// 虚拟地址所对应的Sv39的三级页表项，即第 [38:30],[29:21],[20:12] 位
pub fn pte_idx_of_virt_addr(vaddr: VirtAddr) -> (usize, usize, usize) {
    ((vaddr >> 30) & 0x1ff, (vaddr >> 21) & 0x1ff, (vaddr >> 12) & 0x1ff)
}
