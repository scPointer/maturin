//! 用户程序初始栈信息。
//! 栈的空间排布参考了 `http://articles.manugarg.com/aboutelfauxiliaryvectors.html`

#![allow(unsafe_code)]

use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::null;

use super::flags::*;
use super::InitStack;

/// 初始化信息
pub struct InitInfo {
    /// args strings
    pub args: Vec<String>,
    /// environment strings
    pub envs: Vec<String>,
    /// auxiliary
    pub auxv: BTreeMap<u8, usize>,
}

impl InitInfo {
    /// 将初始信息序列化到栈上
    /// 由栈底(高地址)向栈顶(低地址)依次推入
    pub fn serialize(&self, stack_top: usize) -> InitStack {
        let mut writer = InitStack::new(stack_top);
        // 程序名
        writer.push_str(&self.args[0]);
        // "随机"串。想要真正做到随机需要硬件，但目前实现暂不影响程序运行
        let random_str = &[3703830112808742751usize, 7081108068768079778usize];
        writer.push_slice(random_str.as_slice());
        let random_pos = writer.sp;
        // 环境变量
        let envs: Vec<_> = self
            .envs
            .iter()
            .map(|item| writer.push_str(item.as_str()))
            .collect();
        // 执行参数
        let argv: Vec<_> = self
            .args
            .iter()
            .map(|item| writer.push_str(item.as_str()))
            .collect();
        // 辅助参数
        writer.push_slice(&[null::<u8>(), null::<u8>()]);
        for (&type_, &value) in self.auxv.iter() {
            //info!("auxv {} {:x}", type_ ,value);
            match type_ {
                AT_RANDOM => writer.push_slice(&[type_ as usize, random_pos]),
                _ => writer.push_slice(&[type_ as usize, value]),
            };
        }
        // 环境变量的指针数组
        writer.push_slice(&[null::<u8>()]);
        writer.push_slice(envs.as_slice());
        // 执行参数的指针数组
        writer.push_slice(&[null::<u8>()]);
        writer.push_slice(argv.as_slice());
        // 参数个数
        writer.push_slice(&[argv.len()]);
        writer
    }
}
