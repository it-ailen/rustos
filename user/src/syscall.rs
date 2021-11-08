pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    // 编译器无法判定 asm 是否安全，所以需要用 unsafe 包装起来
    unsafe {
        // risc-V 规定，使用 ecall 完成系统调用
        // 使用 a0~a6(x10~x16) 传递参数，a0(x17) 传递 syscall id
        // 使用 a0~a1 传递返回值
        /*
        llvm_asm!(assembly template
            : output operands
            : input operands
            : clobbers； 用于告知编译器汇编代码会造成的一些影响，避免编译器在不知情的情况下误优化
            : options
        );
            */
        llvm_asm!("ecall"
            : "={x10}" (ret) // 只有一个输出，使用 a0 传递，输出须用 = 开头
            // 输入为 a0~a6，用 a7 指定调用号。{} 用于将寄存器和变量联系起来
            : "{x10}" (args[0]), "{x11}" (args[1]), "{x12}" (args[2]), "{x17}" (id)
            : "memory" // 告知编译器本汇编代码会修改内存
            : "volatile" // 告知编译器需要将此汇编代码原样放在输出文件中，即不做任何优化
        );
    }
    ret
}

const SYSCALL_DUP: usize = 24;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    // syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, 2])
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(SYSCALL_READ, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(xstate: i32) -> isize {
    syscall(SYSCALL_EXIT, [xstate as usize, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

/// 返回当前 ms 数。由于没有时钟对齐，这里只是机器 的系统开机时间
pub fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn sys_exec(path: &str, args: &[*const u8]) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, args.as_ptr() as usize, 0])
}

/// 等待子进程结束
/// pid: -1 表示任意子进程结束；
/// exit_code：进程退出码。
///
/// 返回值：
/// -2 表示子进程存在但尚未结束。
// 进程通过 exit 退出后，它所占用的资源不会立即回收。系统只是回收部分，并将进程标记为僵尸进程。
// waitpid 可以触发回收，并等待直到进程完全退出。
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

pub fn sys_pipe(pipe: &mut [usize]) -> isize {
    syscall(SYSCALL_PIPE, [pipe.as_mut_ptr() as usize, 0, 0])
}

/// 功能：当前进程关闭一个文件。
/// 参数：fd 表示要关闭的文件的文件描述符。
/// 返回值：如果成功关闭则返回 0 ，否则返回 -1 。可能的出错原因：传入的文件描述符并不对应一个打开的文件。
/// syscall ID：57
pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

/// 功能：打开一个常规文件，并返回可以访问它的文件描述符。
/// 参数：path 描述要打开的文件的文件名（简单起见，文件系统不需要支持目录，所有的文件都放在根目录 / 下），
/// flags 描述打开文件的标志，具体含义下面给出。
/// 返回值：如果出现了错误则返回 -1，否则返回打开常规文件的文件描述符。可能的错误原因是：文件不存在。
/// syscall ID：56
pub fn sys_open(path: &str, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}