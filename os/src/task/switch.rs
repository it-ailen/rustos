global_asm!(include_str!("switch.S"));

extern "C" {
    /// 实现在 switch.S 中。完成分配并备份 current_task_cx_ptr2；
    /// 并将控制流切换到 next_task_cx_ptr2 指向的 TaskContext 环境中
    pub fn __switch(current_task_cx_ptr2: *const usize, next_task_cx_ptr2: *const usize);
}