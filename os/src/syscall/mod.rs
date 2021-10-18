mod fs;
mod process;

const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    match id {
        SYSCALL_READ => fs::sys_read(args[0], args[1] as _, args[2]),
        SYSCALL_WRITE => fs::sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => process::sys_exit(args[0] as i32),
        SYSCALL_YIELD => process::sys_yield(),
        SYSCALL_GET_TIME => process::sys_get_time(),
        SYSCALL_GETPID => process::sys_getpid(),
        SYSCALL_FORK => process::sys_fork(),
        SYSCALL_EXEC => process::sys_exec(args[0] as _),
        SYSCALL_WAITPID => process::sys_waitpid(args[0] as _, args[1] as _),
        _ => panic!("Unsupported syscall_id: {}", id),
    }
}
