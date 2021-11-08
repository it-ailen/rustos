use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{config::MEMORY_END, mm::PhysAddr};

use super::address::PhysPageNum;

/// 利用 RAII 思想，负责处理物理页的初始化及回收
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// 根据 ppn 获取被初始化的 页桢
    pub fn new(ppn: PhysPageNum) -> Self {
        // RAII 原则，分配即初始化，这里初始化为0
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

/// Drop trait 用于实现 RAII，即回收时将其持有的数据一起回收，Box 等也是用这种方法
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

/// 页桢管理器，负责物理页的分配和回收
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// 栈方式实现的物理页桢分配器
pub struct StackFrameAllocator {
    /// 物理页号区间[current, end)  此前均 从未 被分配出去过
    current: usize,
    end: usize,
    /// 以后入先出的方式保存了被回收的物理页号
    // 此时已使用了堆
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    /// 初始化分配器可分配的物理页区间
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        println!("last {} Physical Frames.", self.end - self.current);
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    /// 从页桢资源池中分配需要的页
    /// 算法：
    /// 1. 优先从回收栈中选取一页返回，如果有，则直接返回；
    /// 2. 从[current, end)中分配第一页后返回
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }

    /// 回收到 recycled 链表中。
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// 全局页桢分配器，目前使用栈分配方式实现
    /// 采用 Mutex 获取可变性
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> =
        Mutex::new(FrameAllocatorImpl::new());
}

/// 利用全局页桢分配器分配一个物理页桢
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .lock()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

/// 回收页桢
pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().dealloc(ppn);
}

/// 初始化全局页桢分配器，以 [ekernel, MEMORY_END) 为可用区域
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.lock().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}
