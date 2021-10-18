use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::mm::{KERNEL_SPACE, MapPermission, VirtAddr};

/// pid 的 RAII 模式
pub struct PidHandle(pub usize);

/// pid 分配器
pub struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl PidAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: Vec::new(),
        }
    }

    /// 分配新的 pid
    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    /// 回收 pid
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            self.recycled.iter().find(|ppid| **ppid == pid).is_none(),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

lazy_static! {
    /// 全局 PID 分配器
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

/// 分配新 pid
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}

/// 返回应用在内核中的内核栈(虚拟地址)位置，[bottom, top)
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    // 程序内核栈间留一个 "Page" 的 gap，防止写到其它程序的数据上。
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

/// 应用进程在内核的栈，目前按 app_id *(KERNEL_STACK_SIZE + PAGE_SIZE) 的
/// 步长放在跳板下面，主要存放 TaskContext
pub struct KernelStack {
    /// 进程 PID
    pid: usize,
}

impl KernelStack {
    /// 根据 pid 新建一个 KernelStack，
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (bottom, top) = kernel_stack_position(pid);
        KERNEL_SPACE.lock().insert_framed_area(
            bottom.into(),
            top.into(),
            MapPermission::R | MapPermission::W,
        );
        KernelStack { pid }
    }

    /// 在栈顶插入类型为 T 的数据, 并返回其指针
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let top = self.get_top();
        let ptr_mut = (top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }

    /// 返回进程对应的内核栈栈顶
    pub fn get_top(&self) -> usize {
        let (_, top) = kernel_stack_position(self.pid);
        top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}