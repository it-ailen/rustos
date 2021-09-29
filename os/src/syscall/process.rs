use crate::{batch::run_next_app, println};

pub fn sys_exit(state: i32) -> isize {
    println!("[kernel] Application exited with code {}", state);
    // 退出则处理下一个任务
    run_next_app()
}
