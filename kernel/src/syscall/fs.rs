//! 与读写、文件相关的系统调用

#![deny(missing_docs)]

use crate::arch::{get_cpu_id};
use crate::arch::stdin::getchar;
use crate::task::{exit_current_task, suspend_current_task};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// 从 fd 读一个字串，最长为 len，放入 buf 中
pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "[cpu {}] In sys_read, len > 1 is NOT supported.", get_cpu_id());
            let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
            slice[0] = loop {
                let c = getchar();
                if c == 0 {
                    // 如果串口目前没有东西可读，则应该先让给其他进程
                    // 这个设计是为了不让一个核因为等待输入阻塞在此

                    // 但如果切换任务时内核输出调试信息，就可能导致屏幕上一直显示内核的输出，给实际操作的"用户"造成疑惑
                    //suspend_current_task();
                    continue;
                } else {
                    break c;
                }
            };
            // 此处应返回实际读到的字节数，但因为目前只支持 getchar，所以直接返回 1
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}

/// 写一个字串到 fd。这个串放在 buf 中，长为 len
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            //println!("syscall write at {:x}, len = {}", buf as usize, len);
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let string = core::str::from_utf8(slice).unwrap();
            print!("{}", string);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
