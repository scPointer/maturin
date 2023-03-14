//! 全局只有一个的页帧分配器。
//!

use lock::Mutex;

use super::defs::PageFrameConfig;
use super::{PhantomData, Range, Vec};

// 16M bit * (4K per page) = max 64G
// 即最大可适用 64G 内存的分配
type FrameAllocatorImpl = bitmap_allocator::BitAlloc16M;
use bitmap_allocator::BitAlloc;

/// 分配器全局只有一个，用互斥锁保护
static FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> = Mutex::new(FrameAllocatorImpl::DEFAULT);

/// 使用特定 Config 定义的页帧分配器
#[derive(Debug)]
pub struct FrameAllocatorWrapper<Config: PageFrameConfig> {
    _marker: PhantomData<Config>,
}

impl<Config: PageFrameConfig> FrameAllocatorWrapper<Config> {
    /// 指定页帧分配器对应的物理地址区间，
    /// 必须至少在启动时调用一次。
    ///
    /// # Safety
    /// 除了该分配器分配出的页帧之外， `regions` 指定的区间不应以其他任何方式读写。
    /// 这一点需要调用者来保证，因而是 unsafe 的。
    pub unsafe fn init(regions: Vec<Range<usize>>) {
        let mut ba = FRAME_ALLOCATOR.lock();
        for region in regions {
            let frame_start = Config::phys_addr_to_frame_idx(region.start);
            let frame_end = Config::phys_addr_to_frame_idx(region.end - 1) + 1;
            assert!(frame_start < frame_end, "illegal range for frame allocator");
            ba.insert(frame_start..frame_end);
        }
        //println!("frame allocator init end.");
    }

    /// 分配一个页帧
    pub unsafe fn alloc_frame() -> Option<usize> {
        let ret = FRAME_ALLOCATOR
            .lock()
            .alloc()
            .map(Config::frame_idx_to_phys_addr);
        //println!("Allocate frame: {:x?}", ret);
        ret
    }

    /// 分配一段连续的页帧，并要求偏移为 PAGE_SIZE * (1 << align_log2)
    ///
    /// 在 OS 中一般的类型只要求虚拟地址连续，但是一些设备，如 virt 块设备的 buffer 需要物理地址连续，
    /// 所以需要有这个函数
    pub unsafe fn alloc_frame_contiguous(frame_count: usize, align_log2: usize) -> Option<usize> {
        let ret = FRAME_ALLOCATOR
            .lock()
            .alloc_contiguous(frame_count, align_log2)
            .map(Config::frame_idx_to_phys_addr);
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
    pub unsafe fn dealloc_frame(target: usize) {
        //println!("Deallocate frame: {:x}", target);
        FRAME_ALLOCATOR
            .lock()
            .dealloc(Config::phys_addr_to_frame_idx(target))
    }

    /// 回收一段连续的页帧
    pub unsafe fn dealloc_frame_contiguous(target: usize, frame_count: usize) {
        //println!("Deallocate {} frames: {:x}", frame_count, target);
        let start_idx = Config::phys_addr_to_frame_idx(target);
        let mut ba = FRAME_ALLOCATOR.lock();
        for i in start_idx..start_idx + frame_count {
            ba.dealloc(i)
        }
    }
}
