//! 初始化时用到的栈。
//! 这里封装了序列化时将不同数据结构推入栈的过程

use alloc::vec::Vec;
use core::mem::{align_of, size_of};
use core::ops::Deref;

use crate::constants::USER_INIT_STACK_SIZE;

/// 用户程序初始栈，从高向低伸展
pub struct InitStack {
    /// 当前栈顶(相对低地址)
    pub sp: usize,
    /// 栈底(高地址)
    pub buttom: usize,
    /// 内部保存的已序列化的信息
    pub data: Vec<u8>,
}

impl InitStack {
    /// create a stack
    pub fn new(sp: usize) -> Self {
        let mut data = Vec::with_capacity(USER_INIT_STACK_SIZE);
        unsafe {
            data.set_len(USER_INIT_STACK_SIZE);
        }
        InitStack {
            sp,
            buttom: sp,
            data,
        }
    }
    /// 向栈中推入一个序列
    pub fn push_slice<T: Copy>(&mut self, vs: &[T]) {
        self.sp -= vs.len() * size_of::<T>();
        self.sp -= self.sp % align_of::<T>();
        assert!(self.buttom - self.sp <= self.data.len());
        let offset = self.data.len() - (self.buttom - self.sp);
        unsafe {
            core::slice::from_raw_parts_mut(self.data.as_mut_ptr().add(offset) as *mut T, vs.len())
        }
        .copy_from_slice(vs);
    }
    /// 向栈中推入一个串，并返回推入后栈顶(函数内部加了结尾 '\0')
    pub fn push_str(&mut self, s: &str) -> usize {
        self.push_slice(&[b'\0']);
        self.push_slice(s.as_bytes());
        self.sp
    }
}

impl Deref for InitStack {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let offset = self.data.len() - (self.buttom - self.sp);
        &self.data[offset..]
    }
}
