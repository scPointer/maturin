//! 映射到内存的文件系统抽象。即将对一段内存的读写包装成对文件系统的读写
//!
//! 这个文件系统可能是：
//! 1. 它是被 qemu 映射到内存的，这样对其的读写就由 qemu 负责更新到实际的文件
//! 2. 它是在 fs.S 中指定，然后在初始化内核页表时被引入的，这样对其的读写不会涉及原文件。换句话说，对它的写操作只会写在内存里

//#![deny(missing_docs)]

mod device;
mod wrapper;

use crate::{
    constants::{DEVICE_END, DEVICE_START},
    memory::phys_to_virt,
};
use fatfs::{DefaultTimeProvider, FileSystem, FsOptions, LossyOemCpConverter};
use fscommon::BufStream;

pub use device::MemoryMappedDevice;
pub use wrapper::IoWrapper;

pub type IoType = IoWrapper<BufStream<MemoryMappedDevice>>;

/// 创建文件系统实例
pub fn new_memory_mapped_fs() -> FileSystem<IoType, DefaultTimeProvider, LossyOemCpConverter> {
    let device = MemoryMappedDevice::new(phys_to_virt(DEVICE_START), phys_to_virt(DEVICE_END));
    let buf_stream = BufStream::new(device);
    let options = FsOptions::new().update_accessed_date(true);
    FileSystem::new(IoWrapper::new(buf_stream), options).unwrap()
}

// 为了方便进行其他操作，放到 ../file/device.rs 里了
/*
lazy_static! {
    static ref MEMORY_FS: Mutex<FileSystem<IoType, DefaultTimeProvider, LossyOemCpConverter>> = new_memory_mapped_fs()
}
*/
