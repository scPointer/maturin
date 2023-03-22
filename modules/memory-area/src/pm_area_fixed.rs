//! 把物理地址段实现为直接分配对应页帧

//#![deny(missing_docs)]

use alloc::sync::Arc;
use core::slice;

use lock::Mutex;

use super::defs::{align_down, align_up, phys_to_virt, PTEFlags, PAGE_SIZE};

/// 直接分配的物理地址段
#[derive(Debug)]
pub struct PmAreaFixed {
    start: usize,
    end: usize,
}

impl PmArea for PmAreaFixed {
    fn size(&self) -> usize {
        self.end - self.start
    }

    fn clone_as_fork(&self) -> Result<Arc<Mutex<dyn PmArea>>> {
        Ok(Arc::new(Mutex::new(Self {
            start: self.start,
            end: self.end,
        })))
    }

    fn get_frame(&mut self, idx: usize, _need_alloc: bool) -> Result<Option<usize>> {
        let paddr = self.start + idx * PAGE_SIZE;
        debug_assert!(paddr < self.end);
        Ok(Some(paddr))
    }

    fn sync_frame_with_file(&mut self, _idx: usize) {}

    fn release_frame(&mut self, _idx: usize) -> Result {
        Ok(())
    }

    fn read(&mut self, offset: usize, dst: &mut [u8]) -> Result<usize> {
        if offset >= self.size() {
            error!(
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

    fn write(&mut self, offset: usize, src: &[u8]) -> Result<usize> {
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

    fn shrink_left(&mut self, new_start: usize) -> Result {
        if new_start < self.end - self.start {
            self.start += new_start;
            Ok(())
        } else {
            Err(OSError::PmArea_ShrinkFailed)
        }
    }

    fn shrink_right(&mut self, new_end: usize) -> Result {
        if new_end < self.end - self.start {
            self.end = self.start + new_end;
            Ok(())
        } else {
            Err(OSError::PmArea_ShrinkFailed)
        }
    }

    fn split(&mut self, left_end: usize, right_start: usize) -> Result<Arc<Mutex<dyn PmArea>>> {
        if left_end <= right_start && right_start < self.end - self.start {
            let old_end = self.end;
            self.end = self.start + left_end;
            Ok(Arc::new(Mutex::new(
                PmAreaFixed::new(self.start + right_start, old_end).unwrap(),
            )))
        } else {
            Err(OSError::PmArea_SplitFailed)
        }
    }
}

impl PmAreaFixed {
    pub fn new(start: usize, end: usize) -> Result<Self> {
        if start >= end {
            error!(
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
        start_paddr: usize,
        end_paddr: usize,
        offset: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> Result<Self> {
        Self::new(
            start_paddr + offset,
            end_paddr + offset,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_paddr, end_paddr)?)),
            name,
        )
    }

    pub fn from_fixed_pma_negative_offset(
        start_paddr: usize,
        end_paddr: usize,
        offset: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> Result<Self> {
        Self::new(
            start_paddr - offset,
            end_paddr - offset,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_paddr, end_paddr)?)),
            name,
        )
    }

    pub fn from_identical_pma(
        start_vaddr: usize,
        end_vaddr: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> Result<Self> {
        Self::new(
            start_vaddr,
            end_vaddr,
            flags,
            Arc::new(Mutex::new(PmAreaFixed::new(start_vaddr, end_vaddr)?)),
            name,
        )
    }
}
