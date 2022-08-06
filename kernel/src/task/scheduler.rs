use super::{TaskControlBlock, ORIGIN_USER_PROC};
use crate::{arch::get_cpu_id, constants::IS_TEST_ENV, file::load_next_testcase};
use alloc::{collections::VecDeque, sync::Arc};
use lock::Mutex;

lazy_static::lazy_static! {
    /// 任务调度器。它是全局的，每次只能有一个核访问它
    /// 它启动时会自动在队列中插入 ORIGIN_USER_PROC 作为第一个用户程序
    pub static ref GLOBAL_TASK_SCHEDULER: Mutex<Scheduler> = {
        let mut scheduler = Scheduler::new();
        if IS_TEST_ENV { // 评测环境下，输入测例
            //load_testcases(&mut scheduler);
            scheduler.push(load_next_testcase().unwrap());
        } else { // 正常情况下，启动初始进程
            scheduler.push(ORIGIN_USER_PROC.clone());
        }
        Mutex::new(scheduler)
    };
}

/// 任务调度器，目前采用 Round-Robin 算法
/// 在 struct 外部会加一个 Mutex 锁
pub struct Scheduler {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Scheduler {
    /// 新建一个空的调度器
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// 添加一个任务到队列中
    pub fn push(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// 从队列中获取一个任务
    pub fn pop(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    /// 返回队列中元素个数
    pub fn size(&self) -> usize {
        self.ready_queue.len()
    }
}

/// 向任务队列里插入一个任务
pub fn push_task_to_scheduler(task: Arc<TaskControlBlock>) {
    GLOBAL_TASK_SCHEDULER.lock().push(task)
}

/// 从任务队列中拿一个任务，返回其TCB。
/// 非阻塞，即如果没有任务可取，则直接返回 None
pub fn fetch_task_from_scheduler() -> Option<Arc<TaskControlBlock>> {
    if IS_TEST_ENV {
        let task = GLOBAL_TASK_SCHEDULER.lock().pop();
        // 测试环境下，测例执行完就不再等待了，因为不会再有新的任务
        if task.is_none() {
            if let Some(new_tcb) = load_next_testcase() {
                return Some(new_tcb);
            }
            info!("[cpu {}] is idle now", get_cpu_id());
            loop {}
        }
        //println!("[cpu {}] get task", get_cpu_id());
        task
    } else {
        GLOBAL_TASK_SCHEDULER.lock().pop()
    }
}
