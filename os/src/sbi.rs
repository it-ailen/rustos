// bootloader/rustsbi-qemu.bin 直接添加的SBI规范实现的二进制代码，给操作系统提供基本支持服务

const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
pub(crate) const SBI_SHUTDOWN: usize = 8;

//
#[inline(always)]
pub(crate) fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) ->usize {
    let mut ret;
    // 此时处于内核特权级
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg1), "{x17}" (which)
            : "memory"
            : "volatile"
        );
    }
    ret
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown!");
}

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}