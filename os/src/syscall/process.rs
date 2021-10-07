use crate::{println, task::{exit_current_and_run_next, suspend_current_and_run_next}, timer::{get_time, get_time_ms}};

pub fn sys_exit(state: i32) -> isize {
    println!("[kernel] Application exited with code {}", state);
    // 退出则处理下一个任务
    exit_current_and_run_next();
    panic!("never here");

}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
    