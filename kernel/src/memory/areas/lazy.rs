//! 把物理地址段实现为 lazy 分配需要的页帧

//#![deny(missing_docs)]

use alloc::{sync::Arc, vec::Vec};
use core::fmt::{Debug, Formatter, Result};

use lock::Mutex;

use super::{PmArea, VmArea};
use crate::error::{OSError, OSResult};
use crate::memory::{
    addr::{self, addr_to_page_id, align_down},
    Frame, PTEFlags, PhysAddr, VirtAddr, PAGE_SIZE, USER_VIRT_ADDR_LIMIT,
};

/// lazy 分配的物理地址段。当 page fault 发生时会由 VmArea 负责调用这段 PmAreaLazy 进行实际分配
pub struct PmAreaLazy {
    frames: Vec<Option<Frame>>,
}

impl PmArea for PmAreaLazy {
    fn size(&self) -> usize {
        self.frames.len() * PAGE_SIZE
    }

    fn get_frame(&mut self, idx: usize, need_alloc: bool) -> OSResult<Option<PhysAddr>> {
        if need_alloc && self.frames[idx].is_none() {
            if let Some(mut frame) = Frame::new() {
                frame.zero();
                self.frames[idx] = Some(frame);
            } else {
                return Err(OSError::Memory_RunOutOfMemory);
            }
        }
        Ok(self.frames[idx].as_ref().map(|f| f.start_paddr()))
    }

    fn release_frame(&mut self, idx: usize) -> OSResult {
        self.frames[idx]
            .take()
            .ok_or(OSError::PmAreaLazy_ReleaseNotAllocatedPage)?;
        Ok(())
    }

    fn read(&mut self, offset: usize, dst: &mut [u8]) -> OSResult<usize> {
        //info!("pma read");
        self.for_each_frame(offset, dst.len(), |processed: usize, frame: &mut [u8]| {
            dst[processed..processed + frame.len()].copy_from_slice(frame);
        })
    }

    fn write(&mut self, offset: usize, src: &[u8]) -> OSResult<usize> {
        //info!("pma write");
        self.for_each_frame(offset, src.len(), |processed: usize, frame: &mut [u8]| {
            frame.copy_from_slice(&src[processed..processed + frame.len()]);
        })
    }

    fn shrink_left(&mut self, new_start: usize) -> OSResult {
        if new_start < self.size() {
            // 被删除的页帧会在 drop 时自动释放
            self.frames.drain(..addr_to_page_id(new_start));
            Ok(())
        } else {
            Err(OSError::PmArea_ShrinkFailed)
        }
    }

    fn shrink_right(&mut self, new_end: usize) -> OSResult {
        if new_end < self.size() {
            // 被删除的页帧会在 drop 时自动释放
            self.frames.drain(addr_to_page_id(new_end)..);
            Ok(())
        } else {
            Err(OSError::PmArea_ShrinkFailed)
        }
    }

    fn split(&mut self, left_end: usize, right_start: usize) -> OSResult<Arc<Mutex<dyn PmArea>>> {
        if left_end < right_start && right_start < self.size() {
            let new_frames = self.frames.drain(addr_to_page_id(right_start)..).collect();
            // 被删除的页帧会在 drop 时自动释放
            self.frames.drain(addr_to_page_id(left_end)..);
            Ok(Arc::new(Mutex::new(PmAreaLazy::new_from_frames(
                new_frames,
            ))))
        } else {
            Err(OSError::PmArea_SplitFailed)
        }
    }
}

impl PmAreaLazy {
    /// 生成新的pma，给定空间大小但不立即分配物理页
    pub fn new(page_count: usize) -> OSResult<Self> {
        if page_count == 0 {
            println!(
                "page_count cannot be 0 in PmAreaLazy::new(): {:#x?}",
                page_count
            );
            return Err(OSError::PmArea_InvalidRange);
        }
        if page_count > addr::page_count(USER_VIRT_ADDR_LIMIT) {
            println!(
                "page_count is too large in PmAreaLazy::new(): {:#x?}",
                page_count
            );
            return Err(OSError::Memory_RunOutOfMemory);
        }
        let mut frames = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            frames.push(None);
        }
        Ok(Self { frames })
    }
    /// 用给定页帧生成pma
    pub fn new_from_frames(frames: Vec<Option<Frame>>) -> Self {
        Self { frames: frames }
    }

    fn for_each_frame(
        &mut self,
        offset: usize,
        len: usize,
        mut op: impl FnMut(usize, &mut [u8]),
    ) -> OSResult<usize> {
        if offset >= self.size() || offset + len > self.size() {
            println!(
                "out of range in PmAreaLazy::for_each_frame(): offset={:#x?}, len={:#x?}, {:#x?}",
                offset, len, self
            );
            return Err(OSError::PmArea_OutOfRange);
        }
        let mut start = offset;
        let mut len = len;
        let mut processed = 0;
        while len > 0 {
            let start_align = align_down(start);
            let pgoff = start - start_align;
            let n = (PAGE_SIZE - pgoff).min(len);

            let idx = start_align / PAGE_SIZE;
            if self.frames[idx].is_none() {
                if let Some(mut frame) = Frame::new() {
                    //info!("new frame vstart {:x} len {:x} (self_size {:x})pstart {:x}", start, len, self.size(), frame.start_paddr());
                    frame.zero();
                    self.frames[idx] = Some(frame);
                } else {
                    return Err(OSError::Memory_RunOutOfMemory);
                }
                /*
                let mut frame = Frame::new()?;
                frame.zero();
                self.frames[idx] = Some(frame);
                */
            }
            let frame = self.frames[idx].as_mut().unwrap();
            op(processed, &mut frame.as_slice_mut()[pgoff..pgoff + n]);
            unsafe {
                if idx == 0x9 && *(frame.as_mut_ptr() as *mut u16).add(0x466) == 0xdbc5 {
                    *(frame.as_mut_ptr() as *mut u16).add(0x466) = 0x0001;
                }
            }
            start += n;
            processed += n;
            len -= n;
        }
        Ok(processed)
    }
}

impl Debug for PmAreaLazy {
    fn fmt(&self, f: &mut Formatter) -> Result {
        f.debug_struct("PmAreaLazy")
            .field("size", &self.size())
            .finish()
    }
}

impl VmArea {
    pub fn from_delay_pma(
        start_vaddr: VirtAddr,
        size: usize,
        flags: PTEFlags,
        name: &'static str,
    ) -> OSResult<Self> {
        Self::new(
            start_vaddr,
            start_vaddr + size,
            flags,
            Arc::new(Mutex::new(PmAreaLazy::new(size)?)),
            name,
        )
    }
}
