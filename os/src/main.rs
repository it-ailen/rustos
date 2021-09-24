#![no_std] // 告知编译器不使用 std 库，而使用 core
#![no_main] // 没有一般意义上的main
#![feature(llvm_asm)]
#![feature(global_asm)] // 此 feature 用于嵌入全局汇编
#![feature(panic_info_message)] // 通过 PanicInfo::message 获取报错信息


// use sbi::{SBI_SHUTDOWN, sbi_call};
// use syscall::sys_exit; //
#[macro_use]
mod lang_items;
// mod syscall;
mod std;
mod sbi;


// fn shutdown() -> ! {
//     sbi_call(SBI_SHUTDOWN, 0, 0, 0);
//     panic!("It should shutdown!");
// }

// include_str! 将同目录下的汇编转化为字符串
// global_asm! 将汇编字符串嵌入代码中
global_asm!(include_str!("entry.asm"));

/// bss 段需要清零才能正常使用，一般应用的 bss 会由操作系统负责清零，但操作系统自身则需要自己处理
fn clear_bss() {
    // sbss/ebss 是 linker.ld 中指定的位置，这里将它声明为 C 函数
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize .. ebss as usize).for_each(|a|{
        unsafe {
            (a as *mut u8).write_volatile(0);
        }
    })
}


// #[no_mangle] 提示编译器不要对函数进行混淆
#[no_mangle]
pub fn rust_main() -> ! {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack();
        fn boot_stack_top();
    }
    clear_bss();
    println!("Hello, world!");
    println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    println!(
        "boot_stack [{:#x}, {:#x})",
        boot_stack as usize, boot_stack_top as usize
    );
    println!(".bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    panic!("Shutdown machine!");
}
