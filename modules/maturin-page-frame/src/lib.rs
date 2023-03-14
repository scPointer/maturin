//! 页帧分配器

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod external {
    pub use std::mem::ManuallyDrop;
    pub use std::{marker::PhantomData, ops::Range, vec::Vec};
}
#[cfg(not(feature = "std"))]
mod external {
    extern crate alloc;
    use alloc::vec::Vec;
    use core::marker::PhantomData;
    use core::mem::ManuallyDrop;
    use core::ops::Range;
}

use external::*;

//#![deny(missing_docs)]

extern crate bitmap_allocator;
extern crate lock;

mod allocator;
mod defs;
pub use defs::PageFrameConfig;
#[cfg(test)]
mod tests;

//struct DefaultConfig;
//impl defs::PageFrameConfig for DefaultConfig {}
type Allocator<Config> = allocator::FrameAllocatorWrapper<Config>;

/// 页帧定义，自动用 new 和 Drop 包装了页帧的分配和回收过程
#[derive(Debug)]
pub struct Frame<Config: PageFrameConfig> {
    start_paddr: usize,
    frame_count: usize,
    _marker: PhantomData<Config>,
}

impl<Config: PageFrameConfig> Frame<Config> {
    /// 初始化页帧分配器。
    ///
    /// 必须在启动时只由一个核调用且全局仅调用一次
    pub unsafe fn init(regions: Vec<Range<usize>>) {
        Allocator::<Config>::init(regions)
    }

    /// 获取并保存一个页帧
    pub fn new() -> Option<Self> {
        unsafe {
            Allocator::<Config>::alloc_frame().map(|start_paddr| Self {
                start_paddr,
                frame_count: 1,
                _marker: PhantomData,
            })
        }
    }

    /// 获取并保存一段连续的页为一个页帧
    pub fn new_contiguous(frame_count: usize, align_log2: usize) -> Option<Self> {
        unsafe {
            Allocator::<Config>::alloc_frame_contiguous(frame_count, align_log2).map(
                |start_paddr| Self {
                    start_paddr,
                    frame_count,
                    _marker: PhantomData,
                },
            )
        }
    }

    /// 从物理地址直接构造一个页帧
    ///
    /// 它在 Drop 时不会回收这个页帧，因为它不是从 new 构造的，也就没有分配过
    pub unsafe fn from_paddr(start_paddr: usize) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Self {
            start_paddr,
            frame_count: 1,
            _marker: PhantomData,
        })
    }

    /// 获取页帧对应的物理地址
    pub fn start_paddr(&self) -> usize {
        self.start_paddr
    }

    /// 获取页帧大小。如果用 new_contiguous 构造，它可能比一个页更大
    pub fn size(&self) -> usize {
        self.frame_count * Config::get_page_size()
    }

    /// 起始地址转常量字符串
    pub fn as_ptr(&self) -> *const u8 {
        Config::phys_addr_to_virt_addr(self.start_paddr) as *const u8
    }

    /// 起始地址转可变字符串
    pub fn as_mut_ptr(&self) -> *mut u8 {
        Config::phys_addr_to_virt_addr(self.start_paddr) as *mut u8
    }

    /// 把每个 Byte 填充成 byte 参数指定的字节
    pub fn fill(&mut self, byte: u8) {
        unsafe { core::ptr::write_bytes(self.as_mut_ptr(), byte, self.size()) }
    }

    /// 清空页面
    pub fn zero(&mut self) {
        self.fill(0)
    }

    /// 起始地址转 slice
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.as_ptr(), self.size()) }
    }

    /// 起始地址转 mut slice
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.size()) }
    }
}

impl<Config: PageFrameConfig> Drop for Frame<Config> {
    fn drop(&mut self) {
        unsafe {
            if self.frame_count == 1 {
                //println!("dealloc page {:x}", self.start_paddr);
                Allocator::<Config>::dealloc_frame(self.start_paddr)
            } else {
                Allocator::<Config>::dealloc_frame_contiguous(self.start_paddr, self.frame_count)
            }
        }
    }
}
