//! 用户地址空间传来的指针，默认是不安全的

use super::MemorySet;
use crate::syscall::ErrorNo;
use lock::MutexGuard;

/// 指向用户空间的指针。
///
/// 仅会在 TryFrom 生成时检查是否合法。
/// 生成这样一个指针必须获取并持有它所在的地址空间的锁，但不会使用它。
/// 在处理这样一个结构的过程中不能中断、切换任务，否则需要使用 `UserData` 等其他结构替代
///
/// 这样一个地址检查需要较大的开销：
/// - 在 try_from 之前，需获取 MemorySet 的 mutex 锁
/// - 在 try_from 中，需要检查结构是否跨页
/// - 在 try_from 中，需要查询 MemorySet 中的 BTree 找到对应区间，并进入页表检查
/// - 如果对应地址确实是应该 lazy alloc 且还没有 alloc，则会：
/// - - 触发页分配器(radix tree形式的bitest)分配物理页
/// - - 写页表并触发对应地址 flush_tlb
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct UserPtr<T>(UserPtrUnchecked<T>);

impl<T> TryFrom<(usize, &mut MutexGuard<'_, MemorySet>)> for UserPtr<T> {
    type Error = ErrorNo;
    fn try_from<'a>(
        (ptr, vm): (usize, &mut MutexGuard<'_, MemorySet>),
    ) -> Result<Self, Self::Error> {
        match vm.manually_alloc_type(ptr as *const T) {
            Ok(_) => Ok(Self(ptr.into())),
            Err(_) => Err(ErrorNo::EFAULT),
        }
    }
}

impl<T> UserPtr<T> {
    #[allow(unused)]
    pub unsafe fn raw(&self) -> *mut T {
        self.0.raw()
    }
}

/// 指向用户空间的裸指针
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct UserPtrUnchecked<T>(*mut T);

impl<T> From<usize> for UserPtrUnchecked<T> {
    fn from(ptr: usize) -> Self {
        UserPtrUnchecked(ptr as _)
    }
}

impl<T> UserPtrUnchecked<T> {
    #[allow(unused)]
    pub unsafe fn raw(&self) -> *mut T {
        self.0
    }
}
