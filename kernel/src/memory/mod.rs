//use alloc::vec::{self, Vec};
use alloc::vec::Vec;
use core::ops::Range;

mod allocator;
mod addr;
#[cfg(target_arch = "riscv64")]
mod page_table;
#[cfg(target_arch = "riscv64")]
mod page_table_impl_rv64_sv39;
//#[cfg(target_arch = "riscv64")]
mod areas;
mod vmm;

use crate::constants::{
    PAGE_SIZE,
    PHYS_VIRT_OFFSET,
    PHYS_MEMORY_OFFSET,
    PHYS_MEMORY_END,
    USER_VIRT_ADDR_LIMIT,
    DEVICE_START,
    DEVICE_END,
    CPU_NUM,
};

use crate::error::{
    OSError,
    OSResult,
};

pub use addr::{
    PhysAddr, 
    VirtAddr,
    align_up,
    align_down,
    virt_to_phys,
    phys_to_virt,
};

pub use allocator::{
    Frame,
    allocator_init,
};

#[cfg(target_arch = "riscv64")]
pub use page_table::{
    MMUFlags, 
    PageTable, 
    PageTableEntry,
    PTETranslator,
};

#[cfg(target_arch = "riscv64")]
pub use page_table_impl_rv64_sv39::{
    RvPageTable,
    RvPTETranslator,
};

pub use areas::{
    VmArea,
};

pub use vmm::{
    MemorySet,
    kernel_page_table_init,
    handle_kernel_page_fault,
    new_memory_set_for_task,
};

pub fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let start = sbss as usize;
    let end = ebss as usize;
    //println!("clear bss {:x} {:x}", start, end);
    let step = core::mem::size_of::<usize>();
    for i in (start..end).step_by(step) {
        unsafe { (i as *mut usize).write(0) };
    }
    //println!("clear bss end");
}


/// 获取从kernel_end的下一页起，至物理内存最后一页的物理页号
pub fn get_phys_memory_regions() -> Vec<Range<usize>> {
    extern "C" {
        fn kernel_end();
    }
    let start = align_up(virt_to_phys(kernel_end as usize));
    let end = PHYS_MEMORY_END;
    vec![start..end]
}

pub fn create_mapping(ms: &mut MemorySet) -> OSResult {
    ms.push(VmArea::from_fixed_pma(
        DEVICE_START,
        DEVICE_END,
        PHYS_VIRT_OFFSET,
        MMUFlags::READ | MMUFlags::WRITE,
        "ramdisk",
    )?)
}
