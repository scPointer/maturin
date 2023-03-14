//! # 页帧分配器
//! 
//! 一个 RAII 的页帧，包装了由 Bitmap 实现的分配器。
//! 
//! ## 使用
//! 
//! 本项目有 `std` 环境和 `no_std` 两个选项。
//! 
//! 如需在 `std` 环境使用，直接在 `Cargo.toml` 引入即可；
//! 如需在内核中使用，则需要选择：
//! ```ignore
//! range-action-map = { default-features = false }
//! ```
//! 
//! 使用步骤：
//! 
//! ```
//! // 1. 设置 Config 函数
//! # use maturin_page_frame::{PageFrameConfig, Frame};
//! struct MyPageFrameConfig;
//! impl PageFrameConfig for MyPageFrameConfig {
//! // 这里可以选择重写任意函数，也可以完全不重写
//! }
//! type MyFrame = Frame<MyPageFrameConfig>;
//! 
//! // 2. 在内核启动时，向页帧分配器中加入任意个物理地址段(后续也可随时添加)
//! unsafe {
//!     MyFrame::init(vec![0x80000000..0x80002000, 0xffff5000..0xffff6000]);
//! }
//! 
//! // 3. 在上面的初始化过程后，任意核在任意时候可以通过 `Frame::new()` 尝试获取页帧，
//! // 然后保存起来(`Frame` 会在 drop 时自动释放)
//! let mut frames: Vec<MyFrame> = vec![];
//! for i in 0..3 {
//!     let frame = MyFrame::new().unwrap();
//!     frames.push(frame);
//! }
//! 
//! // 4. 检查分配情况
//! assert_eq!(frames[0].size(), 0x1000);
//! assert_eq!(frames[0].start_paddr(), 0x80000000);
//! assert_eq!(frames[1].start_paddr(), 0x80001000);
//! assert_eq!(frames[2].start_paddr(), 0xffff5000);
//! assert!(MyFrame::new().is_none());
//! frames.clear();
//! assert!(MyFrame::new().is_some());
//! 
//! ```
//! ## 测试
//! 
//! 本项目来自 `https://github.com/scPointer/maturin`。
//! 
//! 其中 crate 源码在 `https://github.com/scPointer/maturin/tree/master/range-action-map`，
//! 对这个 crate 本身的单元测试在  `https://github.com/scPointer/maturin/tree/master/range-action-map-test`。
//! 
//! 单元测试本身只包含对数据结构本身的测试，不涉及页表和内存分配。实际在内存中的应用见下
//! 
//! ## 应用参考
//! 
//! 见 `https://github.com/scPointer/maturin/blob/master/kernel/src/memory/allocator/frame.rs`

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod external {
    pub use std::mem::ManuallyDrop;
    pub use std::{marker::PhantomData, ops::Range, vec::Vec};
}

#[cfg(not(feature = "std"))]
mod external {
    extern crate alloc;
    pub use alloc::vec::Vec;
    pub use core::marker::PhantomData;
    pub use core::mem::ManuallyDrop;
    pub use core::ops::Range;
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
