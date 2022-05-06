mod block;
pub use block::BLOCK_DEVICE;

pub type BlockDeviceImpl = block::VirtIOBlock;
