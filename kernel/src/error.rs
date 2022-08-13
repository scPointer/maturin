//! 错误类型

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]

/// 内核中报错的类型。
/// 错误类型的名字不一定代表发生错误的模块
pub enum OSError {
    Ok = 0,
    PageTable_FlagUpdateError,
    PageTable_VirtNotFound,
    PageTable_FrameAllocFailed,
    PageTable_PageAlreadyMapped,
    PageTable_UnknownErrorWhenMap,
    PageTable_PageNotMapped,
    PageTable_UnknownErrorWhenUnmap,
    PageTable_RawAccessToPageTable,

    VmArea_InvalidRange,
    VmArea_VmSizeNotEqualToPmSize,
    VmArea_InvalidUnmap,

    // 在areas/mod.rs 与 vmm.rs 都可能检查到
    PageFaultHandler_AccessDenied,
    // 在areas/mod.rs
    // 触发缺页异常的页在页表中已经是valid了
    // 目前没有想到用户程序会如何触发这一条，除非是OS本身设计问题
    PageFaultHandler_TrapAtValidPage,
    // 在vmm.rs
    // 一般是因为找不到地址所对应的 VmArea，
    // 相当于表示用户程序传过来的地址不合法
    PageFaultHandler_Unhandled,

    PmArea_OutOfRange,
    PmArea_InvalidRange,
    PmArea_ShrinkFailed,
    PmArea_SplitFailed,
    PmAreaLazy_ReleaseNotAllocatedPage,

    // 没有空的*物理*页
    Memory_RunOutOfMemory,
    // *虚拟*地址空间中找不到足够长的连续空间
    Memory_RunOutOfConsecutiveMemory,
    // syscall_mmap 需要的地址和内核地址相交
    MemorySet_UserMmapIntersectWithKernel,
    MemorySet_InvalidRange,
    MemorySet_UnmapAreaNotFound,
    Task_MmapLengthDisagree,
    // unmap 一段 VMA 可能会把分成两段
    // 本身不该算是错误，只是目前还没有实现
    MemorySet_PartialUnmap,

    Loader_ParseElfFailed,
    Loader_InvalidSegment,
    Loader_InvalidSection,
    Loader_AppNotFound,
    Loader_CanNotParseInterpreter,
    Loader_PhdrNotFound,
    Loader_Skipped,

    Task_NoTrapHandler,
    // 申请 physical memory 中的物理页面失败
    Task_RunOutOfMemory,

    // cpu 找不到刚刚切换出来的任务
    CpuLocal_SwitchedFromEmptyTask,

    // 文件描述符已满，无法再分配了
    FdManager_NoAvailableFd,
    // 找不到要求的文件描述符
    FdManager_FdNotFound,
}

pub type OSResult<T = ()> = Result<T, OSError>;
