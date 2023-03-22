//! 一段访问权限相同的物理地址

/// 一段访问权限相同的物理地址。注意物理地址本身不一定连续，只是拥有对应长度的空间
///
/// 可实现为 lazy 分配
pub trait PmArea: core::fmt::Debug + Send + Sync {
    /// 地址段总长度
    fn size(&self) -> usize;
    /// 复制一份区间，新区间结构暂不分配任何实际页帧。一般是 fork 要求的
    fn clone_as_fork(&self) -> Result<Arc<Mutex<dyn PmArea>>>;
    /// 获取 idx 所在页的页帧。
    ///
    /// 如果有 need_alloc，则会在 idx 所在页未分配时尝试分配
    fn get_frame(&mut self, idx: usize, need_alloc: bool) -> Result<Option<usize>>;
    /// 同步页的信息到后端文件中
    fn sync_frame_with_file(&mut self, idx: usize);
    /// 释放 idx 地址对应的物理页
    fn release_frame(&mut self, idx: usize) -> Result;
    /// 读从 offset 开头的一段数据，成功时返回读取长度
    fn read(&mut self, offset: usize, dst: &mut [u8]) -> Result<usize>;
    /// 把数据写到从 offset 开头的地址，成功时返回写入长度
    fn write(&mut self, offset: usize, src: &[u8]) -> Result<usize>;
    /// 从左侧缩短一段(new_start是相对于地址段开头的偏移)
    fn shrink_left(&mut self, new_start: usize) -> Result;
    /// 从右侧缩短一段(new_end是相对于地址段开头的偏移)
    fn shrink_right(&mut self, new_end: usize) -> Result;
    /// 分成三段区间(输入参数都是相对于地址段开头的偏移)
    /// 自己保留[start, left_end), 删除 [left_end, right_start)，返回 [right_start, end)
    fn split(&mut self, left_end: usize, right_start: usize) -> Result<Arc<Mutex<dyn PmArea>>>;
}
