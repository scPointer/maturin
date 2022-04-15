//! Architecture independent page table traits and helpers.

use core::mem::ManuallyDrop;

use super::{PhysAddr, VirtAddr, OSError};

pub type PTETranslator = super::RvPTETranslator;

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
