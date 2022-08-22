//! 一些不知道该放进哪个模块里的通用工具

//#![deny(missing_docs)]

use alloc::string::String;
use alloc::vec::Vec;

/// 获取一个裸指针指向的字符串长度
///
/// 函数会从 start 往后不断读取内存，直到遇到 0 为止。
/// 所以如果字符串没有以 \0 结尾，函数就有可能读到其他内存。
pub unsafe fn get_str_len(start: *const u8) -> usize {
    let mut ptr = start as usize;
    while *(ptr as *const u8) != 0 {
        ptr += 1;
    }
    ptr - start as usize
}

/// 从一个裸指针获取一个 &str 类型
///
/// 注意这个函数没有复制字符串本身，只是换了个类型
pub unsafe fn raw_ptr_to_ref_str(start: *const u8) -> &'static str {
    let len = get_str_len(start);
    // 因为这里直接用用户空间提供的虚拟地址来访问，所以一定能连续访问到字符串，不需要考虑物理地址是否连续
    let slice = core::slice::from_raw_parts(start, len);
    if let Ok(s) = core::str::from_utf8(slice) {
        s
    } else {
        println!("not utf8 slice");
        for c in slice {
            print!("{c} ");
        }
        println!("");
        &"p"
    }
}

/// 从一个裸指针获取一个 String 类型
///
/// 注意这个函数复制了字符串本身，所以返回的数据是在内核里的。
/// 调用者必须保证内存空间足够以及 start 这个地址指向的是个合法的字符串
pub unsafe fn raw_ptr_to_string(start: *const u8) -> String {
    String::from(raw_ptr_to_ref_str(start))
}

/// 从一个字符串指针数组(一般是用户程序执行时的参数)获取所有字符串，存入一个 Vec 中
///
/// 注意这个函数复制了字符串本身，所以返回的数据是在内核里的。
/// 如果按C89的描述，传入的 str_ptr 的类型是 char**
pub unsafe fn str_ptr_array_to_vec_string(str_ptr: *const usize) -> Vec<String> {
    let mut strs = vec![];
    let mut ptr_now = str_ptr;
    while *ptr_now != 0 {
        // println!("ptr_now {:x}, {:x}", ptr_now as usize, *ptr_now as usize);
        // str_ptr 是个指向指针数组的指针，*ptr_now 是一个指向字符数组的指针
        strs.push(raw_ptr_to_string(*ptr_now as *const u8));
        ptr_now = ptr_now.add(1);
    }
    strs
}
