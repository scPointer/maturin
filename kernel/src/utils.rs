//! 一些不知道该放进哪个模块里的通用工具

#![deny(missing_docs)]

/// 获取一个裸指针指向的字符串长度
/// 
/// 函数会从 start 往后不断读取内存，直到遇到 0 为止。
/// 所以如果字符串没有以 \0 结尾，函数就有可能读到其他内存。
pub unsafe fn get_str_len(start: *const u8) -> usize {
    let mut ptr = start as usize;
    while *(ptr as *const u8) != 0 {
        ptr += 1;
    };
    ptr - start as usize
}