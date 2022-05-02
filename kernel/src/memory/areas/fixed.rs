use alloc::sync::Arc;
use core::slice;

//use spin::Mutex;
use lock::Mutex;

use super::{PmArea, VmArea};
use crate::error::{OSError, OSResult};
use crate::memory::{
    addr::{align_down, align_up},
    PTEFlags, PhysAddr, VirtAddr, PAGE_SIZE,
};

/// A PMA representing a fixed physical memory region.
#[derive(Debug)]
pub struct PmAreaFixed {
    start: PhysAddr,
    end: PhysAddr,
}

impl PmArea for PmAreaFixed {
    fn size(&self) -> usize {
        self.end - self.start
    }
    fn get_frame(&mut self, idx: usize, _need_alloc: bool) -> OSResult<Option<PhysAddr>> {
        let paddr = self.start + idx * PAGE_SIZE;
        debug_assert!(paddr < self.end);
        Ok(Some(paddr))
    }
    fn release_frame(&mut self, _idx: usize) -> OSResult {
        Ok(())
    }
    fn read(&mut self, offset: usize, dst: &mut [u8]) -> OSResult<usize> {
        if offset >= self.size() {
            println!(
                "out of range in PmAreaFixed::read(): offset={:#x?}, {:#x?}",
                offset, self
            );
            return Err(OSError::PmArea_OutOfRange);
        }
        let len = dst.len().min(self.end - offset);
        let data = unsafe { slice::from_raw_parts((self.start + offset) as *const u8, len) };
        dst.copy_from_slice(data);
        Ok(len)
    }
    fn write(&mut self, offset: usize, src: &[u8]) -> OSResult<usize> {
        if offset >= self.size() {
            println!(
                "out of range in PmAreaFixed::write(): offset={:#x?}, {:#x?}",
                offset, self
            );
            return Err(OSError::PmArea_OutOfRange);
        }
        let len = src.len().min(self.end - offset);
        let data = unsafe { slice::from_raw_parts_mut((self.start + offset) as *mut u8, len) };
        data.copy_from_slice(src);
        Ok(len)
    }
}

impl PmAreaFixed {
    pub fn new(start: PhysAddr, end: PhysAddr) -> OSResult<Self> {
        if start >= end {
            println!(
                "invalid memory region in PmAreaFixed::new(): [{:#x?}, {:#x?})",
                start, end
            );
            return Err(OSError::PmArea_InvalidRange);
        }
        Ok(Self {
            start: align_down(start),
            end: align_up(end),
        })
    }
}

impl VmArea {
    pub fn from_fixed_pma(
        start_paddr: VirtAddr,
        end_paddr: VirtAddr,
        offset: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> OSResult<Self> {
        Self::new(
            start_paddr + offset,
            end_paddr + offset,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_paddr, end_paddr)?)),
            name,
        )
    }

    pub fn from_fixed_pma_negative_offset(
        start_paddr: VirtAddr,
        end_paddr: VirtAddr,
        offset: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> OSResult<Self> {
        Self::new(
            start_paddr - offset,
            end_paddr - offset,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_paddr, end_paddr)?)),
            name,
        )
    }

    pub fn from_identical_pma(
        start_vaddr: VirtAddr,
        end_vaddr: VirtAddr,
        flags: PTEFlags,
        name: &'static str,
    ) -> OSResult<Self> {
        Self::new(
            start_vaddr,
            end_vaddr,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_vaddr, end_vaddr)?)),
            name,
        )
    }
}
