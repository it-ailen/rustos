use alloc::{sync::Arc, sync::Weak};
use spin::Mutex;

use crate::{mm::UserBuffer, task::suspend_current_and_run_next};

use super::File;

const RING_BUFFER_SIZE: usize = 32;

#[derive(Clone, Copy, PartialEq)]
pub enum RingBufferStatus {
    FULL,
    EMPTY,
    NORMAL,
}
// 环形缓冲区
pub struct PipeRingBuffer {
    /// 缓冲区
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    /// 写入端引用，用于判断是否已关闭。使用弱引用防止循环引用。
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::EMPTY,
            write_end: None,
        }
    }

    /// 设置写端，增加弱引用
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    /// 当前环形缓冲中拥有的数据量
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::EMPTY {
            0
        } else {
            if self.tail > self.head {
                self.tail - self.head
            } else {
                self.tail + RING_BUFFER_SIZE - self.head
            }
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::NORMAL;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::EMPTY;
        }
        c
    }

    pub fn write_byte(&mut self, c: u8) {
        self.status = RingBufferStatus::NORMAL;
        self.arr[self.tail] = c;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::FULL;
        }
    }

    /// 可写数据量
    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::FULL {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }

    /// 检查写端是否被关闭
    /// 通过将对写端的弱引用进行升级，如果实例已释放，则会得到一个 None
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

/// 管道，用于进程间通信。它将实现文件 Trait，这样可以通过文件方式对它进行读写
pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
}

impl Pipe {
    /// 从已有的管道创建读端
    pub fn read_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }

    /// 从已有管道创建写端
    pub fn write_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

/// 创建 pipe，并返回读端和写端
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.lock().set_write_end(&write_end);
    (read_end, write_end)
}

impl File for Pipe {
    fn read(&self, user_buf: crate::mm::UserBuffer) -> usize {
        assert_eq!(self.readable, true);
        let mut buf_iter = user_buf.into_iter();
        let mut read_size = 0usize;
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                // 对管道来说，写端已经关闭的情况下，则可以关闭了
                if ring_buffer.all_write_ends_closed() {
                    return read_size;
                }
                // 由于下一句会切换进程，这里的上下文被切走，ring_buffer 的锁不会被
                // 释放，所以需要手动释放一下
                drop(ring_buffer);
                // 当前 IO 未准备好，先释放 CPU
                suspend_current_and_run_next();
                continue;
            }
            // 最多读 loop_read 个字节
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe {
                        *byte_ref = ring_buffer.read_byte();
                    }
                    read_size += 1;
                } else {
                    return read_size;
                }
            }
        }
    }

    fn write(&self, user_buf: crate::mm::UserBuffer) -> usize {
        assert_eq!(self.writable, true);
        let mut buf_iter = user_buf.into_iter();
        let mut write_size = 0usize;
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_write = ring_buffer.available_write();
            if loop_write == 0 {
                // 由于下一句会切换进程，这里的上下文被切走，ring_buffer 的锁不会被
                // 释放，所以需要手动释放一下
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..loop_write {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    write_size += 1;
                } else {
                    return write_size;
                }
            }
        }
    }
}
