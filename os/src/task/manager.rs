use alloc::{collections::VecDeque, sync::Arc};
use spin::Mutex;

use super::task::TCB;
use lazy_static::lazy_static;

pub struct TaskManager {
    /// 就绪队列
    /// 使用 Arc 是为了减少对 TCB 结构的数据拷贝开销；在一些情况下会更方便
    ready_queue: VecDeque<Arc<TCB>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    /// 添加可运行 TCB
    pub fn add(&mut self, task: Arc<TCB>) {
        self.ready_queue.push_back(task);
    }

    /// 从就绪列表中获取第一个 TCB
    pub fn fetch(&mut self) -> Option<Arc<TCB>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    /// 全局任务管理器，这种实现只支持多核共享一个 TaskManager，其它实现可能是
    /// 每个核独占一个任务管理器
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

/// 添加一个就绪任务
pub fn add_task(task: Arc<TCB>) {
    TASK_MANAGER.lock().add(task)
}

/// 从任务管理器中获取一个就绪任务
pub fn fetch_task() -> Option<Arc<TCB>> {
    TASK_MANAGER.lock().fetch()
}
