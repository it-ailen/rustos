global_asm!(include_str!("switch.S"));

extern "C" {
    /// 实现在 switch.S 中。完成分配并备份 current_task_cx_ptr2；
    /// 并将控制流切换到 next_task_cx_ptr2 指向的 TaskContext 环境中
    /// 保存当前任务的 taskContext，并加载下一个任务的 TaskContext
    /// 
    /// 
    /// switch实现：
    /// 从当前任务的用户栈中，分配一个 TaskContext 的空间，保存当前上下文，然后将其地址写到 current_task_cx_ptr2 指向的变量中；
    /// 再以 next_task_cx_ptr2 指向的变量值作为当前环境，加载到对应寄存器，并最终改写 SP 使 CPU 指向下一个任务
    pub fn __switch(current_task_cx_ptr2: *const usize, next_task_cx_ptr2: *const usize);
}
