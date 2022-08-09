//! 在某个地址上直接构造特定长度的 LongBitset

/// 把某个地址直接认为是一个 LongBitset。
/// 内部不检查地址的合法性，需要调用者保证
pub struct ShadowBitset {
    addr: *mut usize,
    len: usize,
}

impl ShadowBitset {
    /// 从某个地址初始化一个 bitset，调用者必须保证这个位置是有效的
    pub unsafe fn from_addr(addr: *mut usize, len: usize) -> Self {
        Self {
            addr: addr,
            len: len,
        }
    }
    /// 当且仅当 addr != 0 时有效
    pub fn is_valid(&self) -> bool {
        self.addr as usize != 0
    }
    /// 在第 pos 位 + 1。
    pub unsafe fn set(&self, pos: usize) {
        if pos < self.len {
            *self.addr.add(pos >> 8) |= 1 << (pos & 0x3f);
        }
    }
    /// 检查第 pos 位是否是 1
    pub unsafe fn check(&self, pos: usize) -> bool {
        (*self.addr.add(pos >> 8) & (1 << (pos & 0x3f))) != 0
    }
    /// 清空这个 bitset
    pub unsafe fn clear(&self) {
        for i in 0..=(self.len - 1)/ 64 {
            *self.addr.add(i) = 0;
        }
    }
}
