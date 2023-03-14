//! 内存管理模块

pub mod addr;
mod allocator;
mod areas;
mod page_table;
mod user;
mod vmm;

use crate::{
    constants::{
        DEVICE_END, DEVICE_START, PAGE_SIZE, PHYS_MEMORY_END, PHYS_VIRT_OFFSET,
        USER_VIRT_ADDR_LIMIT,
    },
    error::OSResult,
};
use alloc::vec::Vec;
use core::ops::Range;

pub use addr::*;
pub use allocator::{allocator_init, FdAllocator, Frame, Tid};
pub use page_table::{PTEFlags, PageTable, PageTableEntry};

/*
#[cfg(target_arch = "riscv64")]
pub use page_table_impl_rv64_sv39::{
    RvPageTable,
    RvPTETranslator,
};
*/

pub use areas::{PmArea, PmAreaFixed, PmAreaLazy, VmArea};

pub use vmm::{
    enable_kernel_page_table, handle_kernel_page_fault, new_memory_set_for_task, MemorySet,
};

pub use user::{UserPtr, UserPtrUnchecked};

/// 获取从kernel_end的下一页起，至物理内存最后一页的物理页号
pub fn get_phys_memory_regions() -> Vec<Range<usize>> {
    extern "C" {
        fn kernel_end();
    }
    let start = align_up(virt_to_phys(kernel_end as usize));
    let end = PHYS_MEMORY_END;
    vec![start..end, 0xa000_0000..0xbe00_0000]
}

#[allow(dead_code)]
pub fn create_mapping(ms: &mut MemorySet) -> OSResult {
    ms.push(VmArea::from_fixed_pma(
        DEVICE_START,
        DEVICE_END,
        PHYS_VIRT_OFFSET,
        PTEFlags::READ | PTEFlags::WRITE,
        "ramdisk",
    )?)
}
