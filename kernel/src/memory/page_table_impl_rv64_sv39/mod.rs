use alloc::vec::Vec;
use core::{convert::From, mem::ManuallyDrop};

use riscv::asm::{sfence_vma, sfence_vma_all};
use riscv::paging::{
    PTE,
    MapperFlushable,
    FrameAllocatorFor,
    Mapper,
    PageTable as PT,
    PageTableEntry as RvOriginPTE,
    PageTableFlags as PTF,
};
use riscv::register::satp;

use crate::memory::{phys_to_virt, Frame, PhysAddr, VirtAddr};
use crate::memory::{MMUFlags, PageTable, /* PageTableEntry, */ PHYS_VIRT_OFFSET};
use crate::error::OSError;

use riscv::addr::Address;
mod rv {
    pub use riscv::addr::{
        PhysAddrSv39 as PhysAddr,
        VirtAddrSv39 as VirtAddr,
    };
    pub type Page = riscv::addr::PageWith<VirtAddr>;
    pub type Frame = riscv::addr::FrameWith<PhysAddr>;
    pub use riscv::paging::{FlagUpdateError, MapToError, UnmapError};
}
mod entry;
pub use entry::RvPTETranslator;

#[cfg(target_arch = "riscv64")]
type TopLevelPageTable<'a> = riscv::paging::Rv39PageTable<'a>;

pub struct RvPageTable {
    inner: TopLevelPageTable<'static>,
    root: Frame,
    allocator: PageTableFrameAllocator,
}

impl From<MMUFlags> for PTF {
    fn from(f: MMUFlags) -> Self {
        if f.is_empty() {
            return PTF::empty();
        }
        let mut ret = PTF::VALID;
        if f.contains(MMUFlags::READ) {
            ret |= PTF::READABLE;
        }
        if f.contains(MMUFlags::WRITE) {
            ret |= PTF::WRITABLE;
        }
        if f.contains(MMUFlags::EXECUTE) {
            ret |= PTF::EXECUTABLE;
        }
        if f.contains(MMUFlags::USER) {
            ret |= PTF::USER;
        }
        ret
    }
}

impl From<PTF> for MMUFlags {
    fn from(f: PTF) -> Self {
        let mut ret = MMUFlags::empty();
        if f.contains(PTF::READABLE) {
            ret |= MMUFlags::READ;
        }
        if f.contains(PTF::WRITABLE) {
            ret |= MMUFlags::WRITE;
        }
        if f.contains(PTF::EXECUTABLE) {
            ret |= MMUFlags::EXECUTE;
        }
        if f.contains(PTF::USER) {
            ret |= MMUFlags::USER;
        }
        ret
    }
}

impl From<rv::MapToError> for OSError {
    fn from(err: rv::MapToError) -> Self {
        match err {
            rv::MapToError::FrameAllocationFailed => OSError::PageTable_FrameAllocFailed,
            rv::MapToError::PageAlreadyMapped => OSError::PageTable_PageAlreadyMapped,
            _ => OSError::PageTable_UnknownErrorWhenMap,
        }
    }
}

impl From<rv::UnmapError::<rv::PhysAddr>> for OSError {
    fn from(err: rv::UnmapError::<rv::PhysAddr>) -> Self {
        match err {
            rv::UnmapError::PageNotMapped => OSError::PageTable_PageNotMapped,
            _ => OSError::PageTable_UnknownErrorWhenUnmap,
        }
    }
}

impl From<rv::FlagUpdateError> for OSError {
    fn from(_: rv::FlagUpdateError) -> Self {
        OSError::PageTable_FlagUpdateError
    }
}


impl PageTable for RvPageTable {
    fn new() -> Self {
        let mut root = Frame::new().expect("failed to allocate root frame for page table");
        root.zero();
        let table = unsafe { &mut *(phys_to_virt(root.start_paddr()) as *mut PT) };
        Self {
            inner: TopLevelPageTable::new(table, PHYS_VIRT_OFFSET),
            root,
            allocator: PageTableFrameAllocator::new(),
        }
    }

    unsafe fn from_root(root_paddr: PhysAddr) -> ManuallyDrop<Self> {
        let table = &mut *(phys_to_virt(root_paddr) as *mut PT);
        ManuallyDrop::new(Self {
            inner: TopLevelPageTable::new(table, PHYS_VIRT_OFFSET),
            root: ManuallyDrop::into_inner(Frame::from_paddr(root_paddr)),
            allocator: PageTableFrameAllocator::new(),
        })
    }

    fn current_root_paddr() -> PhysAddr {
        satp::read().ppn() << 12
    }

    unsafe fn set_current_root_paddr(root_paddr: PhysAddr) {
        satp::set(satp::Mode::Sv39, 0, root_paddr >> 12)
    }

    fn flush_tlb(&self, vaddr: Option<VirtAddr>) {
        unsafe {
            if let Some(vaddr) = vaddr {
                sfence_vma(0, vaddr)
            } else {
                sfence_vma_all()
            }
        }
    }

    fn root_paddr(&self) -> PhysAddr {
        self.root.start_paddr()
    }

    fn map_kernel(&mut self) {
        let table = unsafe { &mut *(phys_to_virt(self.root.start_paddr()) as *mut PT) };
        let kernel_table = unsafe { &mut *(phys_to_virt(Self::current_root_paddr()) as *mut PT) };
        #[cfg(target_arch = "riscv64")] // [0xffff_ffff_8000_0000, 0xffff_ffff_ffff_ffff]
        for i in 510..512 {
            table[i].set::<rv::PhysAddr>(kernel_table[i].frame(), kernel_table[i].flags());
        }
    }

    fn get_entry(&mut self, vaddr: VirtAddr) -> Result<&mut RvOriginPTE, OSError> {
    //fn get_entry(&mut self, vaddr: VirtAddr) -> Result<&mut dyn PageTableEntry, OSError> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        Ok(self.inner.ref_entry(page)?)
    }

    fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, flags: MMUFlags) -> Result<(), OSError> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        let frame = rv::Frame::of_addr(rv::PhysAddr::new(paddr));
        self.inner
            .map_to(page, frame, flags.into(), &mut self.allocator)?
            .flush();
        Ok(())
    }

    fn unmap(&mut self, vaddr: VirtAddr) -> Result<(), OSError> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner.unmap(page)?.1.flush();
        Ok(())
    }

    fn protect(&mut self, vaddr: VirtAddr, flags: MMUFlags) -> Result<(), OSError> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner.update_flags(page, flags.into())?.flush();
        Ok(())
    }

    fn query(&mut self, vaddr: VirtAddr) -> Result<PhysAddr, OSError> {
        let page = rv::Page::of_addr(rv::VirtAddr::new(vaddr));
        self.inner
            .translate_page(page)
            .map(|f| f.start_address().as_usize())
            .ok_or(OSError::PageTable_VirtNotFound)
    }
}

struct PageTableFrameAllocator {
    frames: Vec<Frame>,
}

impl PageTableFrameAllocator {
    fn new() -> Self {
        Self { frames: Vec::new() }
    }
}

impl FrameAllocatorFor<rv::PhysAddr> for PageTableFrameAllocator {
    fn alloc(&mut self) -> Option<rv::Frame> {
        Frame::new()
            .map(|f| {
                let ret = rv::Frame::of_addr(rv::PhysAddr::new(f.start_paddr()));
                self.frames.push(f);
                ret
            })
    }
}
