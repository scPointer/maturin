//! 检查 syscall 调用中的死循环。
//! 跟具体系统调用无关，只是为了方便在出现死循环（比如有某种wait的syscall未实现）的时候，
//! 用来提前结束进程，至少保证OS不崩

use super::{sys_exit, SyscallNo};
use lock::Mutex;

/// 一个检测死循环的计数器
const LOOP_LIMIT: usize = 100;
/// 通过计数器退出时的返回值
const LOOP_EXIT_CODE: i32 = -100;
/// 计数器实现
static DEAD_LOOP_CNT: Mutex<LoopCounter> = Mutex::new(LoopCounter {
    cnt: 0,
    limit: LOOP_LIMIT,
});

/// 检查循环次数
struct LoopCounter {
    /// 当前已触发了多少次
    cnt: usize,
    /// 当触发到多少次时，直接结束进程
    limit: usize,
}

impl LoopCounter {
    pub fn count(&mut self) -> bool {
        self.cnt += 1;
        self.cnt >= self.limit
    }

    pub fn clear(&mut self) {
        self.cnt = 0;
    }
}

// 如果发现应用程序可能已经陷入调用syscall的死循环(其他的管不到)，
// 则终止这个进程
pub fn check_dead_loop(syscall_id: usize) {
    // 决定是否结束进程
    let kill_proc = if let Ok(name) = SyscallNo::try_from(syscall_id) {
        if name == SyscallNo::EPOLL_WAIT {
            DEAD_LOOP_CNT.lock().count()
        } else {
            //DEAD_LOOP_CNT.lock().clear();
            false
        }
    } else {
        DEAD_LOOP_CNT.lock().count()
    };
    // 把 kill_proc 单独拆出来是为了不锁住 LoopCounter
    if kill_proc {
        error!("user proc caused an endless loop of syscall. kernel killed it.");
        sys_exit(LOOP_EXIT_CODE)
    }
}

/// 进入新进程时，清空计数器。
/// 目前认为全局只有一个 checker，不处理更多的进程导致的死循环，如pipe
pub fn clear_loop_checker() {
    DEAD_LOOP_CNT.lock().clear();
}
