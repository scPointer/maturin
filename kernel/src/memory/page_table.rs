//! Architecture independent page table traits and helpers.

use core::mem::ManuallyDrop;
use alloc::vec::Vec;
use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::register::satp;

use crate::error::{OSError, OSResult};

use super::{PhysAddr, VirtAddr};
use super::Frame;
use super::{
    page_id_to_addr,
    pte_idx_of_virt_addr,
    phys_to_virt,
};


/*
bitflags! {
    pub struct MMUFlags: usize {
        const DEVICE    = 1 << 0;
        const READ      = 1 << 1;
        const WRITE     = 1 << 2;
        const EXECUTE   = 1 << 3;
        const USER      = 1 << 4;
    }
}

pub trait PageTableEntry {
    fn addr(&self) -> PhysAddr;
    fn flags(&self) -> MMUFlags;
    fn is_present(&self) -> bool;

    fn set_addr(&mut self, paddr: PhysAddr);
    fn set_flags(&mut self, flags: MMUFlags);
    fn clear(&mut self);
}
*/

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const VALID = 1 << 0;
        const READ = 1 << 1;
        const WRITE = 1 << 2;
        const EXECUTE = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESS = 1 << 6;
        const DIRTY = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    pub bits: usize,
}

/// 页表项(查询部分)
impl PageTableEntry {
    pub fn new(paddr: PhysAddr, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: paddr | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn addr(&self) -> PhysAddr {
        (self.bits >> 10 & ((1usize << 44) - 1)) << 12
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::VALID) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::READ) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::WRITE) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::EXECUTE) != PTEFlags::empty()
    }
}

/// 页表项(修改部分)
impl PageTableEntry {
    /// 设置页号，将地址[55:12]位取出作为物理页号，置于页表项[53:10]位
    /// 并保留表项中[7:0]位标志位不变
    pub fn set_addr(&mut self, paddr: PhysAddr) {
        self.bits = ((paddr >> 12 & ((1usize << 44) - 1)) << 10) | (self.bits & 0xff);
    }
    /// 设置[7:0]位标志位，并保持物理页号不变
    pub fn set_flags(&mut self, flags: PTEFlags) {
        self.bits = (self.bits & (!0xff)) | flags.bits as usize;
    }
    /// 设置页号与标志位
    pub fn set_all(&mut self, paddr: PhysAddr, flags: PTEFlags) {
        self.bits = ((paddr >> 12) << 10) | flags.bits as usize;
    }
    /// 申请一个页面，并返回申请到的帧
    /// 如果返回 None 说明内存已满
    pub fn alloc_and_set(&mut self) -> Option<Frame> {
        let mut frame = Frame::new()?;
        //清空页面，可能是比较耗时的一点
        frame.zero();
        self.set_all(frame.start_paddr(), PTEFlags::VALID);
        Some(frame)
    }
    /// 清空表项
    pub fn clear(&mut self) {
        self.bits = 0;
    }
}

/// 获取 paddr 页面上的第 idx 个页表项
/// 因为 "paddr 页的内容是页表" 需要调用者保证，所以是unsafe
unsafe fn get_pte_at(paddr: PhysAddr, idx: usize) -> &'static mut PageTableEntry {
    unsafe {
        ((phys_to_virt(paddr) + core::mem::size_of::<usize>() * idx) as *mut PageTableEntry).as_mut().unwrap()
    }
}

/// page table structure
pub struct PageTable {
    root_paddr: PhysAddr,
    frames: Vec<Frame>,
}

/// 页表数据结构本身操作
impl PageTable {
    // 建立页表，并申请一个根页面
    pub fn new() -> OSResult<Self> {
        if let Some(mut frame) = Frame::new(){
            frame.zero();
            Ok(PageTable {
                root_paddr: frame.start_paddr(),
                frames: vec![frame],
            })
        } else {
            Err(OSError::PageTable_FrameAllocFailed)
        }
    }
    /// 从表根地址中生成页表
    pub unsafe fn from_root(paddr: PhysAddr) -> Self {
        Self {
            root_paddr: paddr,
            frames: Vec::new(),
        }
    }
    /// 获取页表项中的物理地址，如页表项为空则新申请一个页面
    fn get_addr_create(&mut self, pte: &mut PageTableEntry) -> Option<PhysAddr> {
        if !pte.is_valid() {
            if let Some(frame) = pte.alloc_and_set() {
                self.frames.push(frame);
            } else {
                return None
            }
        }
        Some(pte.addr())
    }
    /// 查找一个页表项，如为空则新建页面
    fn find_pte_create(&mut self, vaddr: VirtAddr) -> Option<&mut PageTableEntry> {
        let (line0, line1, line2) = pte_idx_of_virt_addr(vaddr);
        //查第一级页表
        let pte = unsafe { get_pte_at(self.root_paddr, line0) };
        let paddr = self.get_addr_create(pte)?;
        //println!("pte {:x}, paddr {:x}", pte.bits, paddr);
        //查第二级页表
        let pte = unsafe { get_pte_at(paddr, line1) };
        let paddr = self.get_addr_create(pte)?;
        //println!("pte {:x}, paddr {:x}", pte.bits, paddr);
        //查第三级页表
        unsafe { Some(get_pte_at(paddr, line2)) }
    }
    /// 查找一个页表项，不申请新页面
    fn find_pte(&self, vaddr: VirtAddr) -> Option<&mut PageTableEntry> {
        let (line0, line1, line2) = pte_idx_of_virt_addr(vaddr);
        //查第一级页表
        let pte = unsafe { get_pte_at(self.root_paddr, line0) };
        if !pte.is_valid() { return None; }
        let paddr = pte.addr();
        //查第二级页表
        let pte = unsafe { get_pte_at(paddr, line1) };
        if !pte.is_valid() { return None; }
        let paddr = pte.addr();
        //查第三级页表
        unsafe { Some(get_pte_at(paddr, line2)) }
    }
    /// 映射一对地址
    #[allow(unused)]
    pub fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: PTEFlags) -> OSResult {
        if let Some(pte) = self.find_pte_create(vaddr) {
            if pte.is_valid() {
                println!("vaddr {:x} is mapped before mapping", vaddr);
                Err(OSError::PageTable_PageAlreadyMapped)
            } else {
                pte.set_all(paddr, flags | PTEFlags::VALID);
                Ok(())
            }
        } else {
            Err(OSError::PageTable_FrameAllocFailed)
        }
    }
    /// 取消映射
    #[allow(unused)]
    pub fn unmap(&mut self, vaddr: VirtAddr) -> OSResult {
        if let Some(pte) = self.find_pte(vaddr) {
            if pte.is_valid() {
                println!("vaddr {:x} is invalid before unmapping", vaddr);
                Err(OSError::PageTable_PageNotMapped)
            } else {
                pte.clear();
                Ok(())
            }
        } else {
            Err(OSError::PageTable_VirtNotFound)
        }
    }
    /// 手动查询页表
    pub fn translate(&self, vaddr: VirtAddr) -> Option<PageTableEntry> {
        self.find_pte(vaddr).map(|pte| *pte)
    }
    /// 生成该页表对应的 satp 寄存器的值(使用Sv39模式)
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_paddr
    }
}

/// 页表的与硬件相关的功能
impl PageTable {
    pub fn get_entry(&self, vaddr: VirtAddr) -> Option<&mut PageTableEntry> {
        self.find_pte(vaddr)
    }

    pub fn query(&mut self, vaddr: VirtAddr) -> Option<PhysAddr> {
        Some(self.find_pte(vaddr)?.addr())
    }

    pub fn current_root_paddr() -> PhysAddr {
        satp::read().ppn() << 12
    }

    pub unsafe fn set_current_root_paddr(root_paddr: PhysAddr) {
        satp::set(satp::Mode::Sv39, 0, root_paddr >> 12)
    }

    pub fn flush_tlb(&self, vaddr: Option<VirtAddr>) {
        unsafe {
            if let Some(vaddr) = vaddr {
                sfence_vma(0, vaddr)
            } else {
                sfence_vma_all()
            }
        }
    }

    pub fn get_root_paddr(&self) -> PhysAddr {
        self.root_paddr
    }

    pub fn current() -> Self {
        unsafe { Self::from_root(Self::current_root_paddr()) }
    }

    /// # Safety
    ///
    /// This function is unsafe because it switches the virtual address space.
    pub unsafe fn set_current(&self) {
        let old_root = Self::current_root_paddr();
        let new_root = self.get_root_paddr();
        println!("switch table {:#x?} -> {:#x?}", old_root, new_root);
        if new_root != old_root {
            Self::set_current_root_paddr(new_root);
            self.flush_tlb(None);
        }
    }
}
/*
pub trait PageTable: Sized {
    fn new() -> Self;

    /// Constructs a multi-level page table from a physical address of the root page table.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the user must ensure that the page table indicated by the
    /// memory region starting from `root_paddr` must has the correct format.
    unsafe fn from_root(root_paddr: PhysAddr) -> ManuallyDrop<Self>;

    fn current_root_paddr() -> PhysAddr;

    /// # Safety
    ///
    /// This function is unsafe because it switches the virtual address space.
    unsafe fn set_current_root_paddr(root_paddr: PhysAddr);

    fn flush_tlb(&self, vaddr: Option<VirtAddr>);

    fn root_paddr(&self) -> PhysAddr;

    fn map_kernel(&mut self);

    fn get_entry(&mut self, vaddr: VirtAddr) -> Result<&mut riscv::paging::PageTableEntry, OSError>;

    fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: MMUFlags) -> Result<(), OSError> {
        let entry = self.get_entry(vaddr)?;
        PTETranslator::set_addr(entry, paddr);
        PTETranslator::set_flags(entry, flags);
        Ok(())
    }

    fn unmap(&mut self, vaddr: VirtAddr) -> Result<(), OSError> {
        PTETranslator::clear(self.get_entry(vaddr)?);
        Ok(())
    }

    fn protect(&mut self, vaddr: VirtAddr, flags: MMUFlags) -> Result<(), OSError> {
        PTETranslator::set_flags(self.get_entry(vaddr)?, flags);
        Ok(())
    }

    fn query(&mut self, vaddr: VirtAddr) -> Result<PhysAddr, OSError> {
        Ok(PTETranslator::addr(self.get_entry(vaddr)?))
    }

    fn current() -> ManuallyDrop<Self> {
        unsafe { Self::from_root(Self::current_root_paddr()) }
    }

    /// # Safety
    ///
    /// This function is unsafe because it switches the virtual address space.
    unsafe fn set_current(&self) {
        let old_root = Self::current_root_paddr();
        let new_root = self.root_paddr();
        println!("switch table {:#x?} -> {:#x?}", old_root, new_root);
        if new_root != old_root {
            Self::set_current_root_paddr(new_root);
            self.flush_tlb(None);
        }
    }
}
*/
