
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

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    // syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, 2])
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(xstate: i32) -> isize {
    syscall(SYSCALL_EXIT, [xstate as usize, 0, 0])
}
