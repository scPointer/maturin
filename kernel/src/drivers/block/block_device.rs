use core::any::Any;
/// 读写块设备的规范
pub trait BlockDevice: Send + Sync + Any {
    ///读一个块到buf
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    ///写一个块
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
