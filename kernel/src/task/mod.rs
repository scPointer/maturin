//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

use lazy_static::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use lock::mutex::Mutex;

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::constants::{CPU_NUM, EMPTY_TASK};
use crate::error::{OSResult, OSError};
use crate::loader::{get_num_app, get_app_data, init_app_cx, init_app_cx_by_entry_and_stack};
use crate::loaders::ElfLoader;
use crate::memory::{MemorySet, new_memory_set_for_task, VirtAddr, PTEFlags};
use crate::arch::get_cpu_id;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: Arc<Mutex<TaskManagerInner>>,
    /// 已经完成任务的核数。不放在inner里检查是为了避免干扰其他核调度
    /// 也不放在panic里是因为：
    ///     默认情况下，只要一个核panic，OS必须停机，方便debug
    ///     而任务调度是特殊情况，所有核调度完才panic，所以在调度里写
    finished_core_cnt: Arc<Mutex<usize>>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
//    current_task: usize,
    current_task_at_cpu: [usize; CPU_NUM],
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        //println!("now");
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        //println!("now");
        for i in 0..num_app {
            let mut vm = new_memory_set_for_task().unwrap();
            let raw_data = get_app_data(i);
            let loader = ElfLoader::new(raw_data).unwrap();
            let args = vec![String::from(".")];
            let (user_entry, user_stack) = loader.init_vm(&mut vm, args).unwrap();
            let trap_cx_ptr_in_kernel_stack = init_app_cx_by_entry_and_stack(i, user_entry, user_stack);
            tasks.push(TaskControlBlock{
                task_cx: TaskContext::goto_restore(trap_cx_ptr_in_kernel_stack),
                task_status: TaskStatus::Ready,
                vm: vm,//MemorySet::new_user()
            });
        }
        //println!("now");
        /*
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            vm: None,
        }; MAX_APP_NUM];
        
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        */
        TaskManager {
            num_app,
            inner: Arc::new(Mutex::new(TaskManagerInner {
                    tasks,
                    current_task_at_cpu: [0; CPU_NUM],
                })),
            finished_core_cnt: Arc::new(Mutex::new(0)),
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) {
        /*
        let mut inner = self.inner.lock();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        */
        let mut inner = self.inner.lock();
        let cpu_id = get_cpu_id();
        // 这里初始化num_app-1，是为了第一次启动的时候通过run_next_task
        // 中的(current + 1..current + self.num_app + 1)来选择任务，也就是相当于从0开始选择
        // 而在多核环境下，一个核启动的第一个应用不一定是task[0]
        inner.current_task_at_cpu[cpu_id] = self.num_app - 1;
        if let Some(next) = self.find_next_task(&inner, cpu_id) {
            println!("[cpu {}] running task {}", cpu_id, next);
            //let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task_at_cpu[cpu_id] = next;
            //let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;

            unsafe {inner.tasks[next].vm.activate(); }
            println!("user activate");
            drop(inner);
            let mut _unused = TaskContext::zero_init();

            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
            }
            // go back to user mode
        } else if inner.current_task_at_cpu[cpu_id] != EMPTY_TASK {
            inner.current_task_at_cpu[cpu_id] = EMPTY_TASK;
            drop(inner);
            let mut cnt = self.finished_core_cnt.lock();
            *cnt += 1;
            if *cnt == CPU_NUM {
                panic!("All applications completed!");
            } else {
                drop(cnt);
            }
            loop {
            }
        } else {
            drop(inner);
            // 已停机，被时钟中断等唤醒，do nothing
            loop {
            }
        }
    }

    /*
    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.lock();
        let cpu_id = get_cpu_id();
        let current = inner.current_task_at_cpu[cpu_id];
        println!("[cpu {}] leaving task {}", cpu_id, current);
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.lock();
        let cpu_id = get_cpu_id();
        let current = inner.current_task_at_cpu[cpu_id];
        println!("[cpu {}] leaving task {}", cpu_id, current);
        inner.tasks[current].task_status = TaskStatus::Exited;
    }
    */

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self, inner: &TaskManagerInner, cpu_id: usize) -> Option<usize> {
        //let inner = self.inner.lock();
        let current = inner.current_task_at_cpu[cpu_id];
        //如果当前cpu已停机，则不再接受新用户程序
        if self.num_app == 0 || inner.current_task_at_cpu[cpu_id] == EMPTY_TASK {
            None
        } else {
            (current + 1..current + self.num_app + 1)
                .map(|id| id % self.num_app)
                .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
        }
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self, new_status_for_current: TaskStatus) {
        let mut inner = self.inner.lock();
        let cpu_id = get_cpu_id();
        //在寻找下一个任务前先修改current状态。这一步需要在inner.lock()保护下进行
        let current = inner.current_task_at_cpu[cpu_id];
        inner.tasks[current].task_status = new_status_for_current;

        if let Some(next) = self.find_next_task(&inner, cpu_id) {
            println!("[cpu {}] leaving task {}", cpu_id, current);
            println!("[cpu {}] running task {}", cpu_id, next);
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task_at_cpu[cpu_id] = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            unsafe {inner.tasks[next].vm.activate(); }
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else if inner.current_task_at_cpu[cpu_id] != EMPTY_TASK {
            println!("[cpu {}] leaving task {}", cpu_id, inner.current_task_at_cpu[cpu_id]);
            inner.current_task_at_cpu[cpu_id] = EMPTY_TASK;
            drop(inner);
            let mut cnt = self.finished_core_cnt.lock();
            *cnt += 1;
            if *cnt == CPU_NUM {
                panic!("All applications completed!");
            } else {
                drop(cnt);
            }
            loop {
            }
        } else {
            drop(inner);
            // 已停机，被时钟中断等唤醒，do nothing
            loop {
            }
        }
    }
    /*
    fn get_mut_vm_now(&self, inner: &'_ TaskManagerInner) -> Option<&'_ mut MemorySet> {
        let cpu_id = get_cpu_id();
        let task_now = inner.current_task_at_cpu[cpu_id];
        if task_now < get_num_app() && task_now >= 0 {
            Some(&mut inner.tasks[task_now].vm)
        } else {
            None
        }
    }
    */
    fn handle_user_page_fault(&self, vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
        let mut inner = self.inner.lock();
        let cpu_id = get_cpu_id();
        let task_now = inner.current_task_at_cpu[cpu_id];
        if task_now < get_num_app() && task_now >= 0 {
            inner.tasks[task_now].vm.handle_page_fault(vaddr, access_flags)
        } else {
            Err(OSError::Task_NoTrapHandler)
        }
    }
}

/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task(new_status_for_current: TaskStatus) {
    TASK_MANAGER.run_next_task(new_status_for_current);
}

/*
/// suspend current task
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// exit current task
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}
*/

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    //mark_current_suspended();
    run_next_task(TaskStatus::Ready);
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    //mark_current_exited();
    run_next_task(TaskStatus::Exited);
}

/// handle user page fault on this hart
pub fn handle_user_page_fault(vaddr: VirtAddr, access_flags: PTEFlags) -> OSResult {
    TASK_MANAGER.handle_user_page_fault(vaddr, access_flags)
}
