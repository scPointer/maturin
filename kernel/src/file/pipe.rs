//! 管道实现
//!
//! 相当于两个文件，其中一个只读，一个只可写，但指向同一片内存。
//! Pipe 的读写可能会触发进程切换。
//! 目前的实现中，Pipe本身分配在堆上

use super::{File, BufferFile, OpenFlags};
use crate::{constants::PIPE_SIZE_LIMIT, task::suspend_current_task};
use alloc::sync::Arc;
use lock::Mutex;

/// 管道内部的 buffer，是个循环队列
struct PipeBuffer {
    data: BufferFile,
    head: usize,
    end: usize,
    len: usize,
}

impl PipeBuffer {
    /// 创建一个 buffer
    pub fn new() -> Self {
        Self {
            data: BufferFile::new(OpenFlags::empty()),
            head: 0,
            end: 0,
            len: 0,
        }
    }
    /// 读尽可能多的内容，注意这个函数不是 trait File 的
    /// 使用 buf[start..]
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let max_len = buf.len().min(self.len);
        self.len -= max_len;
        //println!("set posr {}", self.head);
        // 设置指针到数据前
        unsafe { self.data.set_pos(self.head); }
        // 循环队列不用跨过起点
        if self.head + max_len <= PIPE_SIZE_LIMIT {
            self.data.read_inner(&mut buf[..max_len]);
            self.head += max_len;
        } else { // 需要跨过起点
            self.data.read_inner(&mut buf[..PIPE_SIZE_LIMIT-self.head]);
            unsafe { self.data.set_pos(0); }
            self.data.read_inner(&mut buf[PIPE_SIZE_LIMIT-self.head..max_len]);
            // self.head 后面加的是负数，但它又是 usize，最好不用 +=
            self.head = self.head + max_len - PIPE_SIZE_LIMIT; 
        }
        max_len
    }
    /// 写尽可能多的内容，注意这个函数不是 trait File 的
    fn write(&mut self, buf: &[u8]) -> usize {
        let max_len = buf.len().min(PIPE_SIZE_LIMIT - self.len);
        self.len += max_len;
        // 设置指针到数据后
        unsafe { self.data.set_pos(self.end); }
        // 循环队列不用跨过起点
        if self.end + max_len <= PIPE_SIZE_LIMIT {
            self.data.write_inner(&buf[..max_len]);
            self.end += max_len;
        } else { // 需要跨过起点
            self.data.write_inner(&buf[..PIPE_SIZE_LIMIT-self.end]);
            unsafe { self.data.set_pos(0); }
            self.data.write_inner(&buf[PIPE_SIZE_LIMIT-self.end..max_len]);
            // self.end 后面加的是负数，但它又是 usize，最好不用 +=
            self.end = self.end + max_len - PIPE_SIZE_LIMIT; 
        }
        max_len
    }
    fn get_len(&self) -> usize {
        self.len
    }
}

/// 管道本体，每次创建两份，一个是读端，一个是写端
pub struct Pipe {
    /// 标记是否是读的一端
    is_read: bool,
    /// 管道内保存的数据
    /// 只有所有持有管道的 Arc 被 Drop 时，才会释放其中的 PipeBuffer 的空间
    data: Arc<Mutex<PipeBuffer>>,
}

impl Pipe {
    /// 新建一个管道，返回两端
    pub fn new_pipe() -> (Self, Self) {
        let buf = Arc::new(Mutex::new(PipeBuffer::new()));
        (
            Self {
                is_read: true,
                data: buf.clone(),
            },
            Self {
                is_read: false,
                data: buf,
            },
        )
    }
}

impl File for Pipe {
    /// 读管道中数据
    fn read(&self, buf: &mut [u8]) -> Option<usize> {
        if self.is_read {
            let mut read_len = 0;
            // 先读一次，如果一次完成就不用切换进程了
            read_len += self.data.lock().read(&mut buf[read_len..]);
            info!("read pipe len {}, require {}", read_len, buf.len());
            //let tid = crate::task::get_current_task().unwrap().get_tid_num();
            //if buf.len() != 4 { println!("tid {} read pipe len {}, require {}", tid, read_len, buf.len()); }
            // if buf.len() != read_len { println!("tid {} read {} got {}", tid, buf.len(), read_len); }

            // 如果读够了或者写端的 fd 已经被释放了，则退出
            // 注意这里 self.data 的引用一定是自己持有一个，写端持有一个
            // 就算 fd 被复制，也只是复制 Pipe 外包着的 Arc，内部 self.data 的 Arc 不会复制
            let mut cnt = 0;
            while read_len < buf.len() && Arc::strong_count(&self.data) == 2 {
                cnt += 1;
                if cnt > 2 {
                    break;
                }
                suspend_current_task();
                read_len += self.data.lock().read(&mut buf[read_len..]);
            }
            //if buf.len() != read_len { println!("tid {} read {} final got {}", tid, buf.len(), read_len); }
            //else { println!("tid {} read {} final got {}", tid, buf.len(), read_len); }
            Some(read_len)
        } else {
            None
        }
    }
    /// 写入管道
    fn write(&self, buf: &[u8]) -> Option<usize> {
        if self.is_read {
            None
        } else {
            let mut write_len = 0;
            // 同上，如果一次完成就不用切换进程了
            write_len += self.data.lock().write(&buf[write_len..]);
            /*
            unsafe {
                static mut TIMES: usize = 0;
                TIMES += 1;
                if TIMES % 1000 == 0 {
                    println!("write pipe {} times", TIMES);
                } else if write_len > 4 {
                    println!("write end {} times", TIMES);
                }
            }
            */
            info!("write pipe len {}", write_len);

            //let tid = crate::task::get_current_task().unwrap().get_tid_num();
            //if buf.len() != 4 { println!("tid {} write pipe len {}, require {}", tid, write_len, buf.len()); }
            //if buf.len() != write_len { println!("tid {} write {} got {}", tid, buf.len(), write_len); }
            //else { println!("tid {} write {} got {}", tid, buf.len(), write_len); }

            // 同上，参见 read 函数
            let mut cnt = 0;
            while write_len < buf.len() && Arc::strong_count(&self.data) == 2 {
                cnt += 1;
                if cnt > 2 {
                    break;
                }
                suspend_current_task();
                write_len += self.data.lock().write(&buf[write_len..]);
            }
            //if buf.len() != write_len { println!("tid {} write {} final got {}", tid, buf.len(), write_len); }
            //else { println!("tid {} write {} final got {}", tid, buf.len(), write_len); }
            Some(write_len)
        }
    }
    /// 已准备好读。对于 pipe 来说，这意味着读端的buffer内有值
    fn ready_to_read(&self) -> bool {
        self.is_read && self.data.lock().get_len() > 0
    }
    /// 已准备好写。对于 pipe 来说，这意味着写端的buffer未满
    fn ready_to_write(&self) -> bool {
        !self.is_read && self.data.lock().get_len() < PIPE_SIZE_LIMIT
    }
}
