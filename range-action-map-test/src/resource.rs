//! 模拟一个段内保存的资源，如 pma

use std::vec::Vec;
use std::sync::Mutex;

pub struct FrameAllocator {
    start: usize,
    recycled: Vec<usize>,
}

impl FrameAllocator {
    pub fn new() -> Self {
        Self {
            start: 0,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> usize {
        if let Some(frame) = self.recycled.pop() {
            frame
        } else {
            self.start += 1;
            self.start - 1
        }
    }

    pub fn dealloc(&mut self, frame: usize) {
        self.recycled.push(frame)
    }
}

#[derive(Debug)]
pub struct Frame(usize);

impl Frame {
    pub fn alloc() -> Self {
        Self(FRAME_ALLOCATOR.lock().unwrap().alloc())
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().unwrap().dealloc(self.0);
    }
}

lazy_static::lazy_static! {
    static ref FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new()); 
}
