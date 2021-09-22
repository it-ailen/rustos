use super::{code::*, syscall};


pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(state: i32) -> isize {
    syscall(SYSCALL_EXIT, [state as usize, 0, 0])
}