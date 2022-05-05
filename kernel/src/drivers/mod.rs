mod block;
pub use block::BLOCK_DEVICE;

pub const CLOCK_FREQ: usize = 12500000;
pub const MMIO: &[(usize, usize)] = &[(0x10001000, 0x1000)];
pub type BlockDeviceImpl = block::VirtIOBlock;
