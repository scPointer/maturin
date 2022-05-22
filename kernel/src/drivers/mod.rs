mod block;
mod memory;
pub use block::BLOCK_DEVICE;
pub use memory::new_memory_mapped_fs;

pub type BlockDeviceImpl = block::VirtIOBlock;
pub type MemoryMappedFsIoType = memory::IoType;
