use core::cell::RefCell;

use crate::{
    config::MAX_APP_NUM,
    loader::{get_app_data, get_app_data_by_name, get_num_app},
    trap::TrapContext,
};

use alloc::{sync::Arc, vec::Vec};
pub use context::TaskContext;
use lazy_static::lazy_static;
use switch::__switch;
use task::{TaskStatus, TCB};

pub use self::{
    manager::add_task,
    processor::{schedule, take_current_task},
};

pub use processor::{current_user_token, current_trap_cx, run_tasks, current_task};

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

lazy_static! {
    pub static ref INITPROC: Arc<TCB> =
        Arc::new(TCB::new(get_app_data_by_name("initproc").unwrap()));
}

/// 内核初始化后调用，生成第一个用户程序。
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // 从 Processor 中弹出当前任务
    let task = take_current_task().unwrap();
    // **** hold current PCB lock
    let mut inner = task.acquire_inner_lock();
    //
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    // 所有子任务都挂到 initproc 上去
    {
        let mut initproc_inner = INITPROC.acquire_inner_lock();
        for child in inner.children.iter() {
            child.acquire_inner_lock().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
        // 释放 initproc 锁
    }
    inner.children.clear();
    // 回收用户空间数据页
    inner.memory_set.recycle_data_pages();
    drop(inner); // todo? 为什么要显式 drop 这个 锁?
                 // **** release current PCB lock

    drop(task); // 释放当前任务引用数
    // we do not have to save task context
    // 由于上一任务已经退出，切换时就不需要再保存 taskContext 了。这里将其指定为0
    let _unused: usize = 0;
    // 执行完后，_unused 的值为 *TaskContext 地址，然后我们后面不再使用它了
    schedule(&_unused as *const _);
}

/// *注意*: 这个函数会切换上下文，对持有锁的函数，调用这个函数需要考虑手动释放，避免死锁。
pub fn suspend_current_and_run_next() {
    // 由于是暂停，所以必然有一个正在运行的任务
    let task = take_current_task().unwrap();

    // hold current PCB lock
    let mut task_inner = task.acquire_inner_lock();
    let task_cx_ptr2 = task_inner.get_task_cx_ptr2();
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner); // 释放 PCB 锁

    add_task(task);

    schedule(task_cx_ptr2);
}
