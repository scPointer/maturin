//! 可以存超过 64bit 的扩展版 bitset
//! 
//! 或许它应该和 bitset 一起放到新的模块

use alloc::vec::Vec;
use super::Bitset;

// 由 64bit 的 bitset 组成的长 bitset
#[allow(unused)]
pub struct LongBitset {
    bits: Vec<Bitset>,
    len: usize,
}

impl LongBitset {
    /// 新建一个 bitset，长度为 len，此后的输入范围为[0,len)
    #[allow(unused)]
    pub fn new(len: usize) -> Self {
        Self {
            bits: {
                let mut vec = Vec::new();
                // 存储这个 bitset 需要多少 usize
                vec.resize((len + 0x3f) & 0x3f, Bitset(0));
                vec
            },
            len: len
        }
    }
    /// 在第 pos 位 + 1。
    #[allow(unused)]
    pub fn add(&mut self, pos: usize) {
        if pos < self.len {
            self.bits[pos >> 8].add_bit(pos & 0x3f);
        }
    }
    /// 检查第 pos 位是否是 1
    #[allow(unused)]
    pub fn check(&mut self, pos: usize) -> bool {
        self.bits[pos >> 8].contain_bit(pos & 0x3f)
    }
}
