//! Virtual memory management.

use alloc::collections::{btree_map::Entry, BTreeMap};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result};

use lock::Mutex;

use super::{align_down, align_up, virt_to_phys, phys_to_virt, page_count, VirtAddr};
use super::{VmArea, PmArea, PmAreaLazy, PTEFlags, PageTable};
use super::{
    get_phys_memory_regions,
    create_mapping,
};

use crate::constants::{
    CPU_NUM, 
    PAGE_SIZE, 
    USER_VIRT_ADDR_LIMIT,
    MMIO_REGIONS,
    IS_TEST_ENV,
    IS_PRELOADED_FS_IMG,
    DEVICE_START,
    DEVICE_END,
};
use crate::error::{OSError, OSResult};

/// A set of virtual memory areas with the associated page table.
pub struct MemorySet {
    areas: BTreeMap<usize, VmArea>,
    pt: PageTable,
    is_user: bool,
}

impl MemorySet {
    pub fn new_kernel() -> Self {
        Self {
            areas: BTreeMap::new(),
            pt: PageTable::new().unwrap(),
            is_user: false,
        }
    }

    
    pub fn new_user() -> Self {
        Self {
            areas: BTreeMap::new(),
            pt: PageTable::new().unwrap(),
            is_user: true,
        }
        /*
        let mut pt = PageTable::new().unwrap();
        Self {
            areas: BTreeMap::new(),
            pt,
            is_user: true,
        }
        */
    }
    

    /// Find a free area with hint address `addr_hint` and length `len`.
    /// Return the start address of found free area.
    /// Used for mmap.
    pub fn find_free_area(&self, addr_hint: VirtAddr, len: usize) -> OSResult<VirtAddr> {
        // brute force:
        // try each area's end address as the start
        let addr = core::iter::once(align_up(addr_hint))
            .chain(self.areas.iter().map(|(_, area)| area.end))
            .find(|&addr| self.test_free_area(addr, addr + len))
            .unwrap();
        if addr >= USER_VIRT_ADDR_LIMIT {
            Err(OSError::Memory_RunOutOfConsecutiveMemory)
        } else {
            Ok(addr)
        }
    }

    /// Test whether [`start`, `end`) does not overlap with any existing areas.
    fn test_free_area(&self, start: VirtAddr, end: VirtAddr) -> bool {
        if let Some((_, before)) = self.areas.range(..start).last() {
            if before.is_overlap_with(start, end) {
                return false;
            }
        }
        if let Some((_, after)) = self.areas.range(start..).next() {
            if after.is_overlap_with(start, end) {
                return false;
            }
        }
        true
    }

    /// 插入一段内存段，并将其映射到页表里
    pub fn push(&mut self, vma: VmArea) -> OSResult {
        if !self.test_free_area(vma.start, vma.end) {
            println!("VMA overlap: {:#x?}\n{:#x?}", vma, self);
            return Err(OSError::MemorySet_InvalidRange);
        }
        vma.map_area(&mut self.pt)?;
        self.areas.insert(vma.start, vma);
        Ok(())
    }
    /*
    pub fn init_a_kernel_region(
        &mut self,
        start_vaddr: VirtAddr,
        end_vaddr: VirtAddr,
        offset: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> OSResult {
        self.push(VmArea::from_fixed_pma(
            start_vaddr,
            end_vaddr,
            offset,
            flags,
            name,
        )?)?;
        
        self.push(VmArea::from_identical_pma(
            start_vaddr,
            end_vaddr,
            flags,
            name,
        )?)?;
        
        Ok(())
    }
    */
    /// Remove the area `[start_addr, end_addr)` from `MemorySet`.
    pub fn pop(&mut self, start: VirtAddr, end: VirtAddr) -> OSResult {
        if start >= end {
            println!("invalid memory region: [{:#x?}, {:#x?})", start, end);
            return Err(OSError::MemorySet_InvalidRange);
        }
        let start = align_down(start);
        let end = align_up(end);
        if let Entry::Occupied(e) = self.areas.entry(start) {
            if e.get().end == end {
                e.get().unmap_area(&mut self.pt)?;
                e.remove();
                return Ok(());
            }
        }
        if self.test_free_area(start, end) {
            println!(
                "no matched VMA found for memory region: [{:#x?}, {:#x?})",
                start, end
            );
            Err(OSError::MemorySet_UnmapAreaNotFound)
        } else {
            println!(
                "partially unmap memory region [{:#x?}, {:#x?}) is not supported",
                start, end
            );
            Err(OSError::MemorySet_PartialUnmap)
        }
    }

    /// Handle page fault.
    pub fn handle_page_fault(&mut self, vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
        if let Some((_, area)) = self.areas.range(..=vaddr).last() {
            if area.contains(vaddr) {
                return area.handle_page_fault(vaddr - area.start, access_flags, &mut self.pt);
            }
        }
        println!(
            "unhandled page fault @ {:#x?} with access {:?}",
            vaddr, access_flags
        );
        Err(OSError::PageFaultHandler_Unhandled)
    }

    /// 清空用户段的地址映射
    pub fn clear_user(&mut self) {
        if !self.is_user {
            println!("cannot clear kernel memory set");
            return;
        }
        for area in self.areas.values() {
            if area.is_user() {
                area.unmap_area(&mut self.pt).unwrap();
            }
        }
        self.areas.clear();
    }

    /// 清空用户段的地址映射，但保留内核段的
    pub fn clear_user_and_save_kernel(&mut self) {
        if !self.is_user {
            println!("cannot clear kernel memory set");
            return;
        }
        let mut user_area_start: Vec<usize> = Vec::new();
        for (start, area) in self.areas.iter() {
            if area.is_user() {
                area.unmap_area(&mut self.pt).unwrap();
                user_area_start.push(*start);
            }
        }
        for start in user_area_start.iter() {
            self.areas.remove(start);
        }
    }

    pub fn flush_tlb(&self) {
        self.pt.flush_tlb(None);
    }
    
    /// Activate the associated page table.
    pub unsafe fn activate(&self) {
        self.pt.set_current()
    }

    fn read_write(
        &self,
        start: VirtAddr,
        len: usize,
        access_flags: PTEFlags,
        mut op: impl FnMut(&VmArea, usize, usize, usize) -> OSResult,
    ) -> OSResult {
        let mut start = start;
        let mut len = len;
        let mut processed = 0;
        while len > 0 {
            if let Some((_, area)) = self.areas.range(..=start).last() {
                if area.end <= start {
                    return Err(OSError::MemorySet_InvalidRange);
                }
                if !area.flags.contains(access_flags) {
                    return Err(OSError::PageFaultHandler_AccessDenied);
                }
                let n = (area.end - start).min(len);
                op(area, start - area.start, n, processed)?;
                start += n;
                processed += n;
                len -= n;
            } else {
                return Err(OSError::MemorySet_InvalidRange);
            }
        }
        Ok(())
    }

    pub fn read(
        &self,
        start: VirtAddr,
        len: usize,
        dst: &mut [u8],
        access_flags: PTEFlags,
    ) -> OSResult {
        self.read_write(start, len, access_flags, |area, offset, len, processed| {
            area.pma
                .lock()
                .read(offset, &mut dst[processed..processed + len])?;
            Ok(())
        })
    }

    pub fn write(
        &self,
        start: VirtAddr,
        len: usize,
        src: &[u8],
        access_flags: PTEFlags,
    ) -> OSResult {
        self.read_write(start, len, access_flags, |area, offset, len, processed| {
            area.pma
                .lock()
                .write(offset, &src[processed..processed + len])?;
            Ok(())
        })
    }

    /// 从已有 MemorySet 按照 fork 的要求复制一个新的 MemorySet 。具体来说：
    /// 
    /// 1. 对内核的地址段，所有虚拟地址与物理地址的映射相同
    /// 2. 对用户的地址段，所有虚拟地址和其中的数据相同，但对应的物理地址与 self 中的不同
    pub fn copy_as_fork(&self) -> OSResult<MemorySet> {
        let mut ms = new_memory_set_for_task()?;
        for area in self.areas.values() {
            if area.is_user() {
                ms.push(area.copy_to_new_area_with_data()?)?;
            }
        }
        Ok(ms)
    }
}

impl Drop for MemorySet {
    fn drop(&mut self) {
        //println!("MemorySet drop");
        self.clear_user()
    }
}

impl Debug for MemorySet {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("MemorySet")
            .field("areas", &self.areas.values())
            .field("page_table_root", &self.pt.get_root_paddr())
            .finish()
    }
}

/// Re-build a fine-grained kernel page table, push memory segments to kernel memory set.
fn init_kernel_memory_set(ms: &mut MemorySet) -> OSResult {
    extern "C" {
        fn stext();
        fn etext();
        fn sdata();
        fn edata();
        fn srodata();
        fn erodata();
        fn sbss();
        fn ebss();
        fn idle_stack();
        fn idle_stack_top();
    }
    println!("data end {:x}, stack start {:x}", edata as usize, idle_stack as usize);
    
    use super::PHYS_VIRT_OFFSET;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(stext as usize),
        virt_to_phys(etext as usize),
        PHYS_VIRT_OFFSET,
        PTEFlags::READ | PTEFlags::EXECUTE,
        "ktext",
    )?)?;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(sdata as usize),
        virt_to_phys(edata as usize),
        PHYS_VIRT_OFFSET,
        PTEFlags::READ | PTEFlags::WRITE,
        "kdata",
    )?)?;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(srodata as usize),
        virt_to_phys(erodata as usize),
        PHYS_VIRT_OFFSET,
        PTEFlags::READ | PTEFlags::WRITE | PTEFlags::EXECUTE,
        "krodata",
    )?)?;
    ms.push(VmArea::from_fixed_pma(
        virt_to_phys(sbss as usize),
        virt_to_phys(ebss as usize),
        PHYS_VIRT_OFFSET,
        PTEFlags::READ | PTEFlags::WRITE,
        "kbss",
    )?)?;
    
    // 插入内核栈映射
    let kernel_stack = idle_stack as usize;
    let kernel_stack_top = idle_stack_top as usize;
    let size_per_cpu = (kernel_stack_top - kernel_stack) / CPU_NUM;
    // 这里默认每个核的栈等长，且依次排列在 kernel_stack 中。且默认栈的开头恰好是页面的开头(entry.S中保证)
    for cpu_id in 0..CPU_NUM {
        // 加一页是为了保证内核栈溢出时可以触发异常，而不是跑到其他核的栈去
        let per_cpu_stack_bottom = kernel_stack + size_per_cpu * cpu_id + PAGE_SIZE;
        let per_cpu_stack_top = kernel_stack + size_per_cpu * (cpu_id + 1);
        ms.push(VmArea::from_fixed_pma(
            virt_to_phys(per_cpu_stack_bottom),
            virt_to_phys(per_cpu_stack_top),
            PHYS_VIRT_OFFSET,
            PTEFlags::READ | PTEFlags::WRITE,
            "kstack",
        )?)?;
    }
    
    // 插入物理内存映射
    for region in get_phys_memory_regions() {
        println!("init region {:x}, {:x}", region.start, region.end);
        ms.push(VmArea::from_fixed_pma(
            region.start,
            region.end,
            PHYS_VIRT_OFFSET,
            PTEFlags::READ | PTEFlags::WRITE,
            "physical_memory",
        )?)?;
    }

    // 插入设备的 MMIO 映射
    for region in MMIO_REGIONS {
        // 这里选择恒等映射是为了兼容设备
        ms.push(VmArea::from_identical_pma(
            region.0,
            region.1,
            PTEFlags::READ | PTEFlags::WRITE,
            "MMIO",
        )?)?;
    }

    // 测试环境，需要加载文件系统镜像到内存中
    if IS_TEST_ENV {
        // 如果是测试环境的文件系统镜像，默认qemu已挂载好了不需要
        // 否则，镜像在 data 段里，需要手动把它加载到 device 对应位置
        // 目前的实现不考虑把修改后的文件系统写回的情况
        if !IS_PRELOADED_FS_IMG {
            extern "C" {
                fn img_start();
                fn img_end();
            }
            println!("img start {:x}, img_end {:x}", img_start as usize, img_end as usize);
            let data_len = img_end as usize - img_start as usize;
            let mut pma = PmAreaLazy::new(page_count(data_len))?;
            let data = unsafe { core::slice::from_raw_parts(img_start as *const u8, data_len) };
            pma.write(0, data)?;
            ms.push(VmArea::new(
                phys_to_virt(DEVICE_START),
                phys_to_virt(DEVICE_END),
                PTEFlags::READ | PTEFlags::WRITE,
                Arc::new(Mutex::new(pma)),
                "fs_in_memory"
            )?)?;
        } else {
            // 文件系统的内存映射
            ms.push(VmArea::from_fixed_pma(
                DEVICE_START,
                DEVICE_END,
                PHYS_VIRT_OFFSET,
                PTEFlags::READ | PTEFlags::WRITE,
                "fs_in_memory",
            )?)?;
        }
    }
    //create_mapping(ms)?;
    Ok(())
}

lazy_static::lazy_static! {
    #[repr(align(64))]
    pub static ref KERNEL_MEMORY_SET: Mutex<MemorySet> = {
        let mut ms = MemorySet::new_kernel();
        init_kernel_memory_set(&mut ms).unwrap();
        println!("kernel memory set init end:\n{:#x?}", ms);
        Mutex::new(ms)
    };
}

pub fn handle_kernel_page_fault(vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
    println!(
        "kernel page fault @ {:#x} with access {:?}",
        vaddr, access_flags
    );
    KERNEL_MEMORY_SET.lock().handle_page_fault(vaddr, access_flags)
}

/// Initialize the kernel memory set and activate kernel page table.
pub fn enable_kernel_page_table() {
    unsafe { KERNEL_MEMORY_SET.lock().activate() };
}

pub fn new_memory_set_for_task() -> OSResult<MemorySet> {
    let mut ms = MemorySet::new_user();
    init_kernel_memory_set(&mut ms).unwrap();
    Ok(ms)
}
