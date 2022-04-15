use riscv::paging::{
    PTE,
    PageTableEntry as RvPTE,
    PageTableFlags as PTF,
};

use super::{
    PhysAddr,
    MMUFlags,
    //PageTableEntry,
};

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

/*
pub struct RvPageTableEntry<'a>(&'a mut RvPTE);
impl PageTableEntry for RvPageTableEntry<'_> { 
    fn addr(&self) -> PhysAddr {
        //这里输出 rv::PhysAddr 即可完成类型推断，但因为需要输出OS自己设置的 PhysAddr，所以只能指定类型
        self.0.addr::<rv::PhysAddr>().as_usize()
    }
    fn flags(&self) -> MMUFlags {
        self.0.flags().into()
    }
    fn is_present(&self) -> bool {
        self.0.flags().contains(PTF::VALID)
    }
    fn set_addr(&mut self, paddr: PhysAddr) {
        let frame = rv::Frame::of_addr(rv::PhysAddr::new(paddr));
        self.0.set::<rv::PhysAddr>(frame, self.0.flags())
    }
    fn set_flags(&mut self, flags: MMUFlags) {
        self.0.set::<rv::PhysAddr>(self.0.frame(), flags.into())
    }
    fn clear(&mut self) {
        self.0.set_unused()
    }
}
*/

pub struct RvPTETranslator;
impl RvPTETranslator {
    pub fn addr(p: &mut RvPTE) -> PhysAddr {
        //这里输出 rv::PhysAddr 即可完成类型推断，但因为需要输出OS自己设置的 PhysAddr，所以只能指定类型
        p.addr::<rv::PhysAddr>().as_usize()
    }
    pub fn flags(p: &mut RvPTE) -> MMUFlags {
        p.flags().into()
    }
    pub fn is_present(p: &mut RvPTE) -> bool {
        p.flags().contains(PTF::VALID)
    }
    pub fn set_addr(p: &mut RvPTE, paddr: PhysAddr) {
        let frame = rv::Frame::of_addr(rv::PhysAddr::new(paddr));
        p.set::<rv::PhysAddr>(frame, p.flags())
    }
    pub fn set_flags(p: &mut RvPTE, flags: MMUFlags) {
        p.set::<rv::PhysAddr>(p.frame(), flags.into())
    }
    pub fn clear(p: &mut RvPTE) {
        p.set_unused()
    }
}