use core::cell::RefCell;

use crate::{
    config::MAX_APP_NUM,
    loader::{get_num_app, init_app_cx},
};

pub use context::TaskContext;
use lazy_static;
use switch::__switch;
use task::{TCB, TaskStatus};

mod context;
mod switch;
mod task;

pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>,
}

struct TaskManagerInner {
    /// 任务表。使用数组。序号下标表示其 pid
    tasks: [TCB; MAX_APP_NUM],
    /// 当前正在运行的任务
    current_task: usize,
}

impl TaskManager {
    fn run_first_task(&self) {
        self.inner.borrow_mut().tasks[0].task_status = TaskStatus::Running;
        let next_task_cx_ptr2 = self.inner.borrow().tasks[0].get_task_cx_ptr2();
        let unused: usize = 0;
        unsafe {
            // _ 用于让编译器自动推算
            __switch(&unused as *const _, next_task_cx_ptr2);
        }
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| {
                // 回绕
                id % self.num_app
            })
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr2 = inner.tasks[current].get_task_cx_ptr2();
            let next_task_cx_ptr2 = inner.tasks[next].get_task_cx_ptr2();
            core::mem::drop(inner); // todo 为啥要显式回收？
            unsafe {
                __switch(current_task_cx_ptr2, next_task_cx_ptr2);
            }
        } else {
            panic!("All applications completed!");
        }
    }
}

unsafe impl Sync for TaskManager {}

lazy_static::lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [
            TCB{task_cx_ptr: 0, task_status: task::TaskStatus::UnInit};
            MAX_APP_NUM
        ];
        for i in 0..num_app {
            tasks[i].task_cx_ptr = init_app_cx(i) as *const _ as usize;
            tasks[i].task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner{
                tasks,
                current_task: 0,
            }),
        }
    };
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}
