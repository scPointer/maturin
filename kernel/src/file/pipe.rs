//! 管道实现
//! 
//! 相当于两个文件，其中一个只读，一个只可写，但指向同一片内存。
//! Pipe 的读写可能会触发进程切换。
//! 目前的实现中，Pipe本身分配在堆上

#![deny(missing_docs)]


use alloc::sync::Arc;
use lock::Mutex;
use core::cmp::min;

use crate::memory::Frame;
use crate::constants::PIPE_SIZE;
use crate::task::suspend_current_task;

use super::File;

/// 管道内部的 buffer，是个循环队列
struct PipeBuffer {
    data: [u8; PIPE_SIZE],
    head: usize,
    end: usize,
    len: usize,
}

impl PipeBuffer {
    /// 创建一个 buffer
    pub fn new() -> Self {
        Self {
            data: [0; PIPE_SIZE],
            head: 0,
            end: 0,
            len: 0
        }
    }
    /// 读尽可能多的内容，注意这个函数不是 trait File 的
    /// 使用 buf[start..]
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let max_len = min(buf.len(), self.len);
        self.len -= max_len;
        for i in 0..max_len {
            buf[i] = self.data[self.head];
            self.head = if self.head + 1 < PIPE_SIZE {self.head + 1} else {0};
        }
        max_len
    }
    /// 写尽可能多的内容，注意这个函数不是 trait File 的
    fn write(&mut self, buf: &[u8]) -> usize {
        let max_len = min(buf.len(), PIPE_SIZE - self.len);
        self.len += max_len;
        for i in 0..max_len {
            self.data[self.end] = buf[i];
            self.end = if self.end + 1 < PIPE_SIZE {self.end + 1} else {0};
        }
        max_len
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
            Self {is_read: true, data: buf.clone()},
            Self {is_read: false, data: buf},
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
            println!("read pipe len {}, require {}", read_len, buf.len());
            // 如果读够了或者写端的 fd 已经被释放了，则退出
            // 注意这里 self.data 的引用一定是自己持有一个，写端持有一个
            // 就算 fd 被复制，也只是复制 Pipe 外包着的 Arc，内部 self.data 的 Arc 不会复制
            let mut cnt = 0;
            while read_len < buf.len() && Arc::strong_count(&self.data) == 2 {
                cnt += 1;
                if cnt > 10 {panic!("");}
                suspend_current_task();
                read_len += self.data.lock().read(&mut buf[read_len..]);
            }
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
            println!("write pipe len {}", write_len);
            // 同上，参见 read 函数
            while write_len < buf.len() && Arc::strong_count(&self.data) == 2 {
                suspend_current_task();
                write_len += self.data.lock().write(&buf[write_len..]);
            }
            Some(write_len)
        }
    }
}
