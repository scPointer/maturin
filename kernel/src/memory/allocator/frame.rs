//! 页帧分配器

//#![deny(missing_docs)]

//16M bit * (4K per page) = max 64G
//实际上u740板子的内存16G用不完bit map，但再小一点的实现只有4G空间
extern crate bitmap_allocator;
type FrameAllocatorImpl = bitmap_allocator::BitAlloc16M;
use bitmap_allocator::BitAlloc;

use core::mem::ManuallyDrop;

use lock::Mutex;

use super::phys_to_virt;
use super::{PhysAddr, PAGE_SIZE, PHYS_MEMORY_OFFSET};

/// 分配器全局只有一个，用互斥锁保护
static FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> = Mutex::new(FrameAllocatorImpl::DEFAULT);

/// 物理地址转页帧编号
fn phys_addr_to_frame_idx(addr: PhysAddr) -> usize {
    (addr - PHYS_MEMORY_OFFSET) / PAGE_SIZE
}

/// 页帧编号转物理地址
fn frame_idx_to_phys_addr(idx: usize) -> PhysAddr {
    idx * PAGE_SIZE + PHYS_MEMORY_OFFSET
}

/// 分配一个页帧
///
/// 注意这个函数不对外公开
unsafe fn alloc_frame() -> Option<PhysAddr> {
    let ret = FRAME_ALLOCATOR.lock().alloc().map(frame_idx_to_phys_addr);
    //println!("Allocate frame: {:x?}", ret);
    ret
}

/// 分配一段连续的页帧，并要求偏移为 PAGE_SIZE * (1 << align_log2)
///
/// 在 OS 中一般的类型只要求虚拟地址连续，但是一些设备，如 virt 块设备的 buffer 需要物理地址连续，
/// 所以需要有这个函数
unsafe fn alloc_frame_contiguous(frame_count: usize, align_log2: usize) -> Option<PhysAddr> {
    let ret = FRAME_ALLOCATOR
        .lock()
        .alloc_contiguous(frame_count, align_log2)
        .map(frame_idx_to_phys_addr);
    /*
    println!(
        "Allocate {} frames with alignment {}: {:x?}",
        frame_count,
        1 << align_log2,
        ret
    );
    */
    ret
}

/// 回收一个页帧
///
/// 注意这个函数不对外公开
unsafe fn dealloc_frame(target: PhysAddr) {
    //println!("Deallocate frame: {:x}", target);
    FRAME_ALLOCATOR
        .lock()
        .dealloc(phys_addr_to_frame_idx(target))
}

/// 回收一段连续的页帧
///
/// 注意这个函数不对外公开
unsafe fn dealloc_frame_contiguous(target: PhysAddr, frame_count: usize) {
    //println!("Deallocate {} frames: {:x}", frame_count, target);
    let start_idx = phys_addr_to_frame_idx(target);
    let mut ba = FRAME_ALLOCATOR.lock();
    for i in start_idx..start_idx + frame_count {
        ba.dealloc(i)
    }
}

/// 初始化页帧分配器。
///
/// 必须在启动时只由一个核调用，通常是启动核
pub fn init() {
    let mut ba = FRAME_ALLOCATOR.lock();
    let regions = super::get_phys_memory_regions();
    for region in regions {
        let frame_start = phys_addr_to_frame_idx(region.start);
        let frame_end = phys_addr_to_frame_idx(region.end - 1) + 1;
        assert!(frame_start < frame_end, "illegal range for frame allocator");
        ba.insert(frame_start..frame_end);
    }
    //println!("frame allocator init end.");
}

/// 页帧定义，自动用 new 和 Drop 包装了页帧的分配和回收过程
#[derive(Debug)]
pub struct Frame {
    start_paddr: PhysAddr,
    frame_count: usize,
}

impl Frame {
    /// 获取并保存一个页帧
    pub fn new() -> Option<Self> {
        unsafe {
            alloc_frame().map(|start_paddr| Self {
                start_paddr,
                frame_count: 1,
            })
        }
    }

    /// 获取并保存一段连续的页为一个页帧
    pub fn new_contiguous(frame_count: usize, align_log2: usize) -> Option<Self> {
        unsafe {
            alloc_frame_contiguous(frame_count, align_log2).map(|start_paddr| Self {
                start_paddr,
                frame_count,
            })
        }
    }

    /// 从物理地址直接构造一个页帧
    ///
    /// 它在 Drop 时不会回收这个页帧，因为它不是从 new 构造的，也就没有分配过
    pub unsafe fn from_paddr(start_paddr: PhysAddr) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Self {
            start_paddr,
            frame_count: 1,
        })
    }

    /// 获取页帧对应的物理地址
    pub fn start_paddr(&self) -> PhysAddr {
        self.start_paddr
    }

    /// 获取页帧大小。如果用 new_contiguous 构造，它可能比一个页更大
    pub fn size(&self) -> usize {
        self.frame_count * PAGE_SIZE
    }

    /// 起始地址转常量字符串
    pub fn as_ptr(&self) -> *const u8 {
        phys_to_virt(self.start_paddr) as *const u8
    }

    /// 起始地址转可变字符串
    pub fn as_mut_ptr(&self) -> *mut u8 {
        phys_to_virt(self.start_paddr) as *mut u8
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

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            if self.frame_count == 1 {
                //println!("dealloc page {:x}", self.start_paddr);
                dealloc_frame(self.start_paddr)
            } else {
                dealloc_frame_contiguous(self.start_paddr, self.frame_count)
            }
        }
    }
}
