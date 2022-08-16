//! 虚拟地址段映射管理

use super::{
    addr_to_page_id, align_down, align_up, cross_page, get_phys_memory_regions, page_count,
    page_id_to_addr, virt_to_phys,
    DiffSet, CutSet, PTEFlags, PageTable, PmAreaLazy, VirtAddr, VmArea,
};
use crate::{
    arch,
    file::BackEndFile,
    constants::{
        CPU_ID_LIMIT, DEVICE_END, DEVICE_START, IS_PRELOADED_FS_IMG, IS_TEST_ENV, MMIO_REGIONS,
        PAGE_SIZE, USER_VIRT_ADDR_LIMIT, REPORT_PAGE_FAULT,
    },
    error::{OSError, OSResult},
};
use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
};
use core::{
    fmt::{Debug, Formatter, Result},
    mem::size_of,
};
use lock::Mutex;

/// 内存段和相关的页表
pub struct MemorySet {
    /// 标记内存段的位置
    areas: BTreeMap<usize, VmArea>,
    /// 对应的页表
    pub pt: PageTable,
    /// 是否是用户态的
    is_user: bool,
}

impl MemorySet {
    /// 内核态的映射表
    pub fn new_kernel() -> Self {
        Self {
            areas: BTreeMap::new(),
            pt: PageTable::new().unwrap(),
            is_user: false,
        }
    }

    /// 用户态的映射表
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

    /// 寻找一个起始地址不小于 addr_hint，长为 len 的内存段。找不到时报错
    pub fn find_free_area(&self, hint: VirtAddr, len: usize) -> OSResult<VirtAddr> {
        // 最好不要有一段内存区间从 0 开始
        let test_addr = if hint == 0 { PAGE_SIZE } else { align_up(hint) };
        let addr = core::iter::once(test_addr)
            .chain(self.areas.iter().map(|(_, area)| area.end))
            .find(|&addr| self.test_free_area(addr, addr + len))
            .unwrap();
        if addr + len >= USER_VIRT_ADDR_LIMIT {
            Err(OSError::MemorySet_UserMmapIntersectWithKernel)
        } else {
            Ok(addr)
        }
    }

    /// 检查 [start, end) 是否与其他内存段冲突。
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

    /// 调整所有和已知区间相交的区间，空出 [start, end) 段。
    /// 它可以直接当作 munmap 使用，也可以当作 mmap 的前置操作
    pub fn modify_overlap_areas(&mut self, start: VirtAddr, end: VirtAddr) -> OSResult {
        // 注意，这里把相交的区间直接从 self.areas 里取出来了
        // 所以如果仅相交而不需要删除，就需要放回 self.areas
        let areas_to_be_modified: Vec<VmArea> = self
            .areas
            .drain_filter(|_, area| area.is_overlap_with(start, end))
            .map(|(_, v)| v)
            .collect();
        for mut area in areas_to_be_modified {
            match area.shrink_or_split_if_overlap(&mut self.pt, start, end)? {
                DiffSet::Shrinked => {
                    info!("try shrink to {:x}, {:x}", area.start, area.end);
                    self.areas.insert(area.start, area);
                }
                DiffSet::Splitted(left, right) => {
                    self.areas.insert(left.start, left);
                    self.areas.insert(right.start, right);
                }
                _ => {} // 被删除或者未相交时，就不需要再管了
            }
        }
        Ok(())
    }

    /// 调整所有和已知区间相交的区间，修改 [start, end) 段的权限。
    /// 它可以直接当作 mprotect 使用
    pub fn modify_overlap_areas_with_new_flags(&mut self, start: VirtAddr, end: VirtAddr, new_flags: PTEFlags) -> OSResult {
        // 注意，这里把相交的区间直接从 self.areas 里取出来了
        // 所以如果仅相交而不需要删除，就需要放回 self.areas
        let areas_to_be_modified: Vec<VmArea> = self
            .areas
            .drain_filter(|_, area| area.is_overlap_with(start, end))
            .map(|(_, v)| v)
            .collect();
        for mut area in areas_to_be_modified {
            match area.split_and_modify_if_overlap(&mut self.pt, start, end, new_flags)? {
                CutSet::WholeModified => {
                    //info!("mprotect: modified {:x}, {:x}", area.start, area.end);
                    self.areas.insert(area.start, area);
                }
                CutSet::ModifiedLeft(left, right) | CutSet::ModifiedRight(left, right) => {
                    // 在 split_and_modify_if_overlap 内部已经处理过了修改 flags 的部分
                    // 所以如果有半边相交，可以直接把切出的区间塞回 self.areas
                    //info!("mprotect: cut and modified one-side {:x}, {:x}", area.start, area.end);
                    self.areas.insert(left.start, left);
                    self.areas.insert(right.start, right);
                }
                CutSet::ModifiedMiddle(left, mid, right) => {
                    //info!("mprotect: cut and modified middle {:x}, {:x}", area.start, area.end);
                    self.areas.insert(left.start, left);
                    self.areas.insert(mid.start, mid);
                    self.areas.insert(right.start, right);
                }
                _ => {} // 未相交时，就不需要再管了
            }
        }
        Ok(())
    }

    /// 尝试插入一段数据。如插入成功，返回插入后的起始地址
    ///
    /// 如果指定参数 anywhere，则任意找一段地址 mmap; 否则必须在 [start, end) 尝试插入。
    ///
    /// 输入时默认已保证 start + data.len() == end
    pub fn push_with_backend(
        &mut self,
        start: VirtAddr,
        end: VirtAddr,
        flags: PTEFlags,
        backend: Option<BackEndFile>,
        anywhere: bool,
    ) -> OSResult<usize> {
        let (start, end) = if anywhere {
            let len = end - start;
            // 此处 start 作为 hint
            let start = self.find_free_area(start, end - start)?;
            (start, start + len)
        } else {
            if end >= USER_VIRT_ADDR_LIMIT {
                return Err(OSError::MemorySet_UserMmapIntersectWithKernel);
            }
            // 否则说明要求特定区间，那么需要 unmap 掉原本相交的区间
            self.modify_overlap_areas(start, end).unwrap();
            (start, end)
        };
        //info!("origin start {:x} end {:x}", start, end);
        // 注意实际占用的页数不仅看 data.len()，还要看请求的地址跨越了几页
        let pma = PmAreaLazy::new(page_count(end - start), backend)?;
        //pma.write(0, data)?;
        //info!("before align: start {:x}, end {:x}", start, end);
        let start = align_down(start);
        let end = align_up(end);
        //println!("after align: start {:x}, end {:x}, pmsize {}", start, end, align_up(off + data.len()));
        let area = VmArea::new(start, end, flags, Arc::new(Mutex::new(pma)), "from mmap").unwrap();
        //println!("start {}, end {}", start, end);
        self.push(area)?;
        // 其实可以只刷新 mmap 的页，但因为这里不清楚具体会 mmap 多大内存，所以按页刷不一定划算
        self.flush_tlb();
        Ok(start)
    }

    /// 插入一段内存段，并将其映射到页表里
    pub fn push(&mut self, vma: VmArea) -> OSResult {
        if !self.test_free_area(vma.start, vma.end) {
            info!("VMA overlap: {:#x?}\n{:#x?}", vma, self);
            return Err(OSError::MemorySet_InvalidRange);
        }
        vma.map_area(&mut self.pt)?;
        self.areas.insert(vma.start, vma);
        Ok(())
    }

    /// 处理这个映射表对应的错误
    pub fn handle_page_fault(&mut self, vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
        if let Some((_, area)) = self.areas.range(..=vaddr).last() {
            if area.contains(vaddr) {
                return area.handle_page_fault(vaddr - area.start, access_flags, &mut self.pt);
            }
        }
        if REPORT_PAGE_FAULT {
            println!(
                "unhandled page fault @ {:#x?} with access {:?}",
                vaddr, access_flags
            );
        }
        Err(OSError::PageFaultHandler_Unhandled)
    }

    /// 检查一个地址是否分配，如果未分配则强制分配它
    pub fn manually_alloc_page(&mut self, vaddr: VirtAddr) -> OSResult {
        if let Some((_, area)) = self.areas.range(..=vaddr).last() {
            if area.contains(vaddr) {
                return area.manually_alloc_page(vaddr - area.start, &mut self.pt);
            }
        }
        Err(OSError::PageFaultHandler_Unhandled)
    }

    /// 检查一个放在某个地址上的结构是否分配空间，如果未分配则强制分配它
    pub fn manually_alloc_type<T>(&mut self, user_obj: *const T) -> OSResult {
        let vaddr = user_obj as usize;
        if cross_page::<T>(vaddr) {
            for page in addr_to_page_id(vaddr)..=addr_to_page_id(vaddr + size_of::<T>() - 1) {
                self.manually_alloc_page(page_id_to_addr(page))?;
            }
            Ok(())
        } else {
            self.manually_alloc_page(vaddr)
        }
    }

    /// 检查一段地址是否每一页都已分配空间，如果未分配则强制分配它
    pub fn manually_alloc_range(&mut self, start_vaddr: VirtAddr, end_vaddr: VirtAddr) -> OSResult {
        for page in addr_to_page_id(start_vaddr)..=addr_to_page_id(end_vaddr) {
            self.manually_alloc_page(page_id_to_addr(page))?;
        }
        Ok(())
    }

    /// 检查一段用户地址空间传来的字符串是否已分配空间，如果未分配则强制分配它
    pub fn manually_alloc_user_str(&mut self, buf: *const u8, len: usize) -> OSResult {
        self.manually_alloc_range(buf as VirtAddr, buf as VirtAddr + len - 1)
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

    // 清空 TLB
    pub fn flush_tlb(&self) {
        self.pt.flush_tlb(None);
    }

    /// 切换到这个 MemorySet 内的页表
    pub unsafe fn activate(&self) {
        self.pt.set_current()
    }

    /// 包装读写操作
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

    /// 读操作
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

    /// 写操作
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

/// 初始化 MemorySet，加载所有内存段
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
    }
    let range = arch::kernel_stack();
    let idle_stack = range.start;
    let idle_stack_top = range.end;

    info!(
        "data end {:x}, stack start {:x}",
        edata as usize, idle_stack
    );

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
        virt_to_phys(edata as usize + PAGE_SIZE),
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
    let kernel_stack = idle_stack;
    let kernel_stack_top = idle_stack_top;
    let size_per_cpu = (kernel_stack_top - kernel_stack) / CPU_ID_LIMIT;
    // 这里默认每个核的栈等长，且依次排列在 kernel_stack 中。且默认栈的开头恰好是页面的开头(entry.S中保证)
    for cpu_id in 0..CPU_ID_LIMIT {
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
        info!("init region {:x}, {:x}", region.start, region.end);
        ms.push(VmArea::from_fixed_pma(
            region.start,
            region.end,
            PHYS_VIRT_OFFSET,
            PTEFlags::READ | PTEFlags::WRITE,
            "physical_memory",
        )?)?;
    }

    if !IS_TEST_ENV {
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
    }
    // 测试环境，需要加载文件系统镜像到内存中
    if IS_TEST_ENV {
        if !IS_PRELOADED_FS_IMG {
            extern "C" {
                fn img_start();
                fn img_end();
            }
            info!(
                "img start {:x}, img_end {:x}",
                img_start as usize, img_end as usize
            );
            let pstart = virt_to_phys(img_start as usize);
            let pend = virt_to_phys(img_end as usize);
            let offset = DEVICE_START - pstart;
            // 文件系统的内存映射
            ms.push(VmArea::from_fixed_pma(
                pstart,
                pend,
                PHYS_VIRT_OFFSET + offset,
                PTEFlags::READ | PTEFlags::WRITE,
                "fs_in_memory",
            )?)?;
        } else {
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

/*
/// 加载 data 段中的文件系统。必须在测试环境且非预载文件系统的情况下调用。
/// 即 IS_TEST_ENV && !IS_PRELOADED_FS_IMG
///
/// 仅用于fs出问题无法加载时进行测试
fn load_fs_force() {
    extern "C" {
        fn img_start();
        fn img_end();
    }
    println!(
        "img start {:x}, img_end {:x}",
        img_start as usize, img_end as usize
    );
    let data_len = img_end as usize - img_start as usize;
    // println!("get data");
    let src = unsafe { core::slice::from_raw_parts(img_start as *const u8, data_len) };
    let dst = unsafe { core::slice::from_raw_parts_mut(DEVICE_START as *mut u8, data_len) };
    dst.copy_from_slice(src);
    unsafe {
        println!("data[0]={}", (DEVICE_START as *mut u8).read_volatile());
    }
}
*/

lazy_static::lazy_static! {
    #[repr(align(64))]
    pub static ref KERNEL_MEMORY_SET: Mutex<MemorySet> = {
        let mut ms = MemorySet::new_kernel();
        init_kernel_memory_set(&mut ms).unwrap();
        // 如果是测试环境的文件系统镜像，默认qemu已挂载好了不需要
        // 否则，镜像在 data 段里，需要手动把它加载到 device 对应位置
        // 目前的实现不考虑把修改后的文件系统写回的情况
        /*
        if IS_TEST_ENV && !IS_PRELOADED_FS_IMG {
            load_fs_force();
        }
        */
        info!("kernel memory set init end:\n{:#x?}", ms);
        Mutex::new(ms)
    };
}

/// 处理来自内核的异常中断
/// 当前还没有需要处理的中断
#[allow(dead_code)]
pub fn handle_kernel_page_fault(vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
    println!(
        "kernel page fault @ {:#x} with access {:?}",
        vaddr, access_flags
    );
    KERNEL_MEMORY_SET
        .lock()
        .handle_page_fault(vaddr, access_flags)
}

/// 切换到 KERNEL_MEMORY_SET 中的页表。
/// 每个核启动时都需要调用
pub fn enable_kernel_page_table() {
    unsafe { KERNEL_MEMORY_SET.lock().activate() };
}

/// 为 ms 映射内存段的地址。ms 本身一般是用户态的
fn map_kernel_regions(ms: &mut MemorySet) {
    let kernel_ms = KERNEL_MEMORY_SET.lock();
    let kernel_pt = &kernel_ms.pt;
    unsafe { ms.pt.map_kernel_regions(kernel_pt) };
}

/// 创建一个新的用户进程对应的内存映射。它的内核态可以访问内核态的所有映射，但不能修改
pub fn new_memory_set_for_task() -> OSResult<MemorySet> {
    let mut ms = MemorySet::new_user();
    //init_kernel_memory_set(&mut ms).unwrap();
    map_kernel_regions(&mut ms);
    Ok(ms)
}
