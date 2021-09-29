//! # 全局属性
//! - `#![no_std]`  
//!   禁用标准库
#![no_std] // 告知编译器不使用 std 库，而使用 core
//! - `#![no_main]`  
//!   不使用 `main` 函数等全部 Rust-level 入口点来作为程序入口
#![no_main] // 没有一般意义上的main
//! # 一些 unstable 的功能需要在 crate 层级声明后才可以使用
//! - `#![feature(llvm_asm)]`  
//!   内嵌汇编
#![feature(llvm_asm)]
//! - `#![feature(global_asm)]`  
//!   内嵌整个汇编文件
#![feature(global_asm)] // 此 feature 用于嵌入全局汇编
//! - `#![feature(panic_info_message)]`  
//!   panic! 时，获取其中的信息并打印
#![feature(panic_info_message)] // 通过 PanicInfo::message 获取报错信息


// use sbi::{SBI_SHUTDOWN, sbi_call};
// use syscall::sys_exit; //
mod lang_items;
mod syscall;
mod sbi;
mod batch;
mod trap;
#[macro_use]
mod console;


// fn shutdown() -> ! {
//     sbi_call(SBI_SHUTDOWN, 0, 0, 0);
//     panic!("It should shutdown!");
// }

// include_str! 将同目录下的汇编转化为字符串
// global_asm! 将汇编字符串嵌入代码中
global_asm!(include_str!("entry.asm"));

// 引入应用
global_asm!(include_str!("link_app.S"));

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

pub fn init() {
    extern "C" {
    }
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
    println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    println!(
        "boot_stack [{:#x}, {:#x})",
        boot_stack as usize, boot_stack_top as usize
    );
    println!(".bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    // panic!("Shutdown machine!");
    println!("[kernel] Hello, world!");
    trap::init();
    batch::init();
    batch::run_next_app();
}
