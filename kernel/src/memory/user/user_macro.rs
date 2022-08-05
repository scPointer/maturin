//! 为了方便使用用户指针、数组设置的宏

/// 获取一个用户指针。当不指定 vm(MemorySet) 时，即表示地址就在当前 cpu 的 satp 中指示的页表内。
#[allow(unused_macros)]
macro_rules! user_ptr_from {
    ($addr: ident, $vm: ident) => {
        UserPtr::try_from(($addr, &mut $vm))
    };
}
