use core::cell::RefCell;

use alloc::sync::Arc;

use lazy_static::lazy_static;

use crate::trap::TrapContext;

use super::{
    manager::fetch_task,
    switch::__switch,
    task::{TaskStatus, TCB},
};

/// 处理器管理结构，对应一个 CPU 核
pub struct Processor {
    /// 使用 RefCell 存放运行时可能变化的数据。
    inner: RefCell<ProcessorInner>,
}

/// Processor 是每个核有一个，不管在多核还是单核模式下访问都不会有数据竞争问题，
/// 所以可以标为 Sync
unsafe impl Sync for Processor {}

struct ProcessorInner {
    /// 处理器当前运行的任务
    current: Option<Arc<TCB>>,
    /// idle 空闲控制流：运行在每个核的启动栈上，作用是尝试从任务管理器中选出一个任务来在当前核上执行。
    idle_task_cx_ptr: usize,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ProcessorInner {
                current: None,
                idle_task_cx_ptr: 0,
            }),
        }
    }

    /// 当前处理器 idle 控制流的指针
    fn get_idle_task_cx_ptr2(&self) -> *const usize {
        let inner = self.inner.borrow();
        &inner.idle_task_cx_ptr as *const usize
    }

    /// 获取当前运行任务，并将CPU 的当前任务置为 None。
    /// 即换出当前运行任务。
    pub fn take_current(&self) -> Option<Arc<TCB>> {
        self.inner.borrow_mut().current.take()
    }

    /// 复制当前任务结构。
    pub fn current(&self) -> Option<Arc<TCB>> {
        self.inner
            .borrow()
            .current
            .as_ref()
            .map(|task| Arc::clone(task))
    }

    pub fn run(&self) {
        loop {
            if let Some(task) = fetch_task() {
                // 当前 idle 控制流
                let idle_task_cx_ptr2 = self.get_idle_task_cx_ptr2();
                let mut task_inner = task.acquire_inner_lock();
                let next_task_cx_ptr2 = task_inner.get_task_cx_ptr2();
                task_inner.task_status = TaskStatus::Running;
                // 手动释放互斥锁，不能等到编译器自己回收（会在函数结束后），临界区扩大可能造成死锁
                drop(task_inner);

                self.inner.borrow_mut().current = Some(task);
                // 从 idle 控制流切换至目标任务
                // 执行完 switch 后， self.idle_task_cx_ptr 的值是指向由 switch.S 从当前 run 的栈空间
                // 分配到的 *TaskContext
                unsafe { __switch(idle_task_cx_ptr2, next_task_cx_ptr2) }
            }
        }
    }
}

lazy_static! {
    /// 只实现了单核，所以只需要实例化一个单例
    pub static ref PROCESSOR: Processor = Processor::new();
}

pub fn run_tasks() {
    PROCESSOR.run()
}

/// 换出当前任务的 TCB
pub fn take_current_task() -> Option<Arc<TCB>> {
    PROCESSOR.take_current()
}

/// 获取当前任务
pub fn current_task() -> Option<Arc<TCB>> {
    PROCESSOR.current()
}

/// 获取当前任务的用户空间 token(satp)
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.acquire_inner_lock().get_user_token();
    token
}

/// 当前任务的 TrapContext
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().acquire_inner_lock().get_trap_cx()
}

/// 切换至 idle 控制流并开启新一轮调试。
/// 这里实际上是继续运行 Processor.run 中 __switch 后的位置
/// 执行后，*switched_task_cx_ptr2 = *TaskContext as usize(*TaskContext 是从此进程栈空间分配的)
pub fn schedule(switched_task_cx_ptr2: *const usize) {
    let idle_task_cx_ptr2 = PROCESSOR.get_idle_task_cx_ptr2();
    unsafe {
        __switch(switched_task_cx_ptr2, idle_task_cx_ptr2);
    }
}
