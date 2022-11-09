//! bit 数组，以及支持更长范围的拓展 bit 数组

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
/// bit数组。可取并集和差集，也可对给定的 mask 取首位
pub struct Bitset(pub usize);

impl Bitset {
    /// 新建一个数组，长为 usize = 8Byte
    pub fn new(v: usize) -> Self {
        Self(v)
    }
    /// 直接暴力写入 bitset
    pub fn reset(&mut self, v: usize) {
        self.0 = v;
    }
    /// 清空 bitset
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    /// 是否包含第 k 个 bit
    pub fn contain_bit(&self, kth: usize) -> bool {
        ((self.0 >> kth) & 1) > 0
    }
    /// 新增一个 bit
    pub fn add_bit(&mut self, kth: usize) {
        self.0 |= 1 << kth;
    }
    /// 删除一个 bit
    pub fn remove_bit(&mut self, kth: usize) {
        self.0 &= !(1 << kth);
    }
    /// 取交集
    pub fn get_union(&mut self, set: Bitset) {
        self.0 |= set.0;
    }
    /// 取差集，即去掉 set 中的内容
    pub fn get_difference(&mut self, set: Bitset) {
        self.0 &= !(set.0);
    }
    /// 直接设置为新值
    pub fn set_new(&mut self, set: Bitset) {
        self.0 = set.0;
    }
    /// 获取后缀0个数，可以用来寻找最小的1
    pub fn get_trailing_zeros(&self) -> u32 {
        self.0.trailing_zeros()
    }
    /// 寻找不在mask中的最小的 1 的位置，如果有，返回其位置，如没有则返回 None。
    pub fn find_first_one(&self, mask: Bitset) -> Option<usize> {
        let ans = (self.0 & !mask.0).trailing_zeros() as usize;
        if ans == 64 {
            None
        } else {
            Some(ans)
        }
    }
}

impl From<usize> for Bitset {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

/// 由 64bit 的 bitset 组成的长 bitset
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
            len: len,
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

/// 在某个地址上直接构造特定长度的 LongBitset
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
        for i in 0..=(self.len - 1) / 64 {
            *self.addr.add(i) = 0;
        }
    }
}
