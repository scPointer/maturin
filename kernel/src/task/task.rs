//! 用户程序的数据及状态信息
//! 一个 TaskControlBlock 包含了一个任务(之后会是进程)的所有信息

#![deny(missing_docs)]

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use lock::Mutex;

use crate::loader::{get_app_data};
use crate::loaders::ElfLoader;
use crate::memory::{MemorySet, new_memory_set_for_task};
use crate::trap::TrapContext;
use crate::arch::get_cpu_id;

use super::{TaskContext, KernelStack};

/// 任务控制块，包含一个用户程序的所有状态信息，但不包括与调度有关的信息
// 默认在TCB的外层有 Arc<Mutex<>>，所以内部的结构没有用锁保护
pub struct TaskControlBlock {
    /// 任务执行状态
    pub task_status: TaskStatus,
    /// 上下文信息，用于切换，包含所有必要的寄存器
    /// 实际在第一次初始化时还包含了用户程序的入口地址和用户栈
    pub task_cx: TaskContext,
    /// 任务的内存段(内含页表)，同时包括用户态和内核态
    pub vm: MemorySet,
    /// 用户程序的内核栈，内部包含申请的内存空间
    /// 因为 struct 内部保存了页帧 Frame，所以 Drop 这个结构体时也会自动释放这段内存
    pub kernel_stack: KernelStack,
}


impl TaskControlBlock {
    /// 从 app_id 生成 TCB
    pub fn from_app_id(app_id: usize) -> Self {
        // 获取用户库编译链接好的(elf 格式的)用户数据
        let raw_data = get_app_data(app_id);
        Self::from_elf(app_id, raw_data)
    }
    /// 从ELF文件初始化一个TCB。
    /// 目前的实现返回 Self ，也就是说如果这个过程不成功，则直接 panic
    pub fn from_elf(app_id: usize, raw_data: &[u8]) -> Self {
        // 新建页表，包含内核段
        let mut vm = new_memory_set_for_task().unwrap();
        // 然后插入页表和 VmArea
        let loader = ElfLoader::new(raw_data).unwrap();
        let args = vec![String::from(".")];
        let (user_entry, user_stack) = loader.init_vm(&mut vm, args).unwrap();
        // 初始化内核栈，它包含关于进入用户程序的所有信息
        let kernel_stack = KernelStack::new().unwrap();
        kernel_stack.print_info();
        let stack_top = kernel_stack.push_first_context(TrapContext::app_init_context(user_entry, user_stack));
        TaskControlBlock {
            task_cx: TaskContext::goto_restore(stack_top),
            task_status: TaskStatus::Ready,
            vm: vm,
            kernel_stack: kernel_stack
        }
    }
    /// 从 fork 系统调用初始化一个TCB
    pub fn from_fork(&self) -> ! {
        unimplemented!();
    }
    /// 从 exec 系统调用初始化一个TCB
    pub fn from_exec(&self) -> ! {
        unimplemented!();
    }
}

/// 任务执行状态
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// 还未初始化
    UnInit,
    /// 已初始化但还未执行，可以被任意一个核执行
    Ready, 
    /// 正在被一个核执行
    Running, 
    /// 已执行完成
    Exited, 
}
