//! 系统调用编号

numeric_enum_macro::numeric_enum! {
    #[repr(i32)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    #[allow(non_camel_case_types)]
    /// 内核中报错的类型。
    /// 错误类型的名字不一定代表发生错误的模块
    pub enum OSError {
        Ok = 0,
        PageTable_FlagUpdateError = 1,
        PageTable_VirtNotFound = 2,
        PageTable_FrameAllocFailed = 3,
        PageTable_PageAlreadyMapped = 4,
        PageTable_UnknownErrorWhenMap = 5,
        PageTable_PageNotMapped = 6,
        PageTable_UnknownErrorWhenUnmap = 7,
        PageTable_RawAccessToPageTable = 8,

        VmArea_InvalidRange = 11,
        VmArea_VmSizeNotEqualToPmSize = 12,
        VmArea_InvalidUnmap = 13,

        // 在areas/mod.rs 与 vmm.rs 都可能检查到
        PageFaultHandler_AccessDenied = 21,
        // 在areas/mod.rs
        // 触发缺页异常的页在页表中已经是valid了
        // 目前没有想到用户程序会如何触发这一条，除非是OS本身设计问题
        PageFaultHandler_TrapAtValidPage = 22,
        // 在vmm.rs
        // 一般是因为找不到地址所对应的 VmArea，
        // 相当于表示用户程序传过来的地址不合法
        PageFaultHandler_Unhandled = 23,

        PmArea_OutOfRange = 31,
        PmArea_InvalidRange = 32,
        PmArea_ShrinkFailed = 33,
        PmArea_SplitFailed = 34,
        PmAreaLazy_ReleaseNotAllocatedPage = 35,

        // 没有空的*物理*页
        Memory_RunOutOfMemory = 41,
        // *虚拟*地址空间中找不到足够长的连续空间
        Memory_RunOutOfConsecutiveMemory = 42,
        // syscall_mmap 需要的地址和内核地址相交
        MemorySet_UserMmapIntersectWithKernel = 43,
        MemorySet_InvalidRange = 44,
        MemorySet_UnmapAreaNotFound = 45,
        MemorySet_AreaNotMapped = 46,
        Task_MmapLengthDisagree = 51,

        Loader_ParseElfFailed = 71,
        Loader_InvalidSegment = 72,
        Loader_InvalidSection = 73,
        Loader_AppNotFound = 74,
        Loader_CanNotParseInterpreter = 75,
        Loader_PhdrNotFound = 76,
        Loader_Skipped = 77,

        Task_NoTrapHandler = 81,
        // 申请 physical memory 中的物理页面失败
        Task_RunOutOfMemory = 82,

        // cpu 找不到刚刚切换出来的任务
        CpuLocal_SwitchedFromEmptyTask = 91,

        // 文件描述符已满，无法再分配了
        FdManager_NoAvailableFd = 101,
        // 找不到要求的文件描述符
        FdManager_FdNotFound = 102,
    }
}
