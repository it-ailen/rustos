#![no_std]
#![feature(linkage)]
#![feature(llvm_asm)]
#![feature(panic_info_message)] // 使用 panic message，有这个 panic_handler 才能起作用

use syscall::*;

mod syscall;
#[macro_use] // 使 console 中定义的宏能被此 crate 外使用，比如 bin 下面的各应用
pub mod console;
mod lang_items;

#[no_mangle] // 不允许编译器混淆
#[link_section = ".text.entry"] // 告知编译器将此函数放到 .text.entry 节，成为用户程序入口
pub extern "C" fn _start() -> ! {
    clear_bss(); // 系统还不具有清零 .bss 的能力，需要应用程序自己做
    println!("user lib run now!!!");
    exit(main()); // 调用用户库的 exit 方法
    panic!("unreachable after sys_exit!");
}

// 将 main 标记为弱连接，在bin和 lib 下都有 main，但由于lib是弱连接，它会被链接器忽略，替换为没有标记的（强连接）
// 类似于 default，只是为了让 lib 在没有 bin 的时候也能编译通过
#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

fn clear_bss() {
    extern "C" {
        fn start_bss();
        fn end_bss(); // 对应 linker.ld 中对 .bss 的区域指定
    }
    (start_bss as usize..end_bss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    })
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(xstate: i32) -> isize {
    sys_exit(xstate)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn get_time() -> isize {
    sys_get_time()
}
