//! 内存区间模块所需要的页帧接口

pub trait PageFrameInter: Drop {
    /// 获取页帧
    fn new() -> Option<Self>;
    /// 清空页面内容
    fn zero();
    /// 将页面内容转 slice
    pub fn as_slice(&self) -> &[u8];
    /// 将页面内容转 mut slice
    pub fn as_slice_mut(&mut self) -> &mut [u8];
}
