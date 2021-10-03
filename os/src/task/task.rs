// #[derive(...)] 提供一些 trait 的默认实现
// PartialEq 是实现 == 运算符重载的默认方式
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// 未初始化
    UnInit,
    /// 准备运行
    Ready,
    ///
    Running,
    ///
    Exited,
}

/// 程序控制块，内核记录任务执行状态的结构
#[derive(Clone, Copy)]
pub struct TCB {
    /// TaskContext 的指针
    pub task_cx_ptr: usize,
    /// 任务状态
    pub task_status: TaskStatus,
}

impl TCB {
    /// 获取本 TCB 表示的 TaskContext 指针的引用；
    // __switch 函数需要这个值作为输入，这说明 __switch 操作的 TaskContext 是处于内核空间的
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }
}
