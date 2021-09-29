#![no_std] // target 系统没有 std
#![no_main] // 不使用常规的 main
#![feature(llvm_asm)] // llvm_asm 是不稳定的 feature

#[macro_use] // 使用user_lib 中的宏 println! 等
extern crate user_lib; // 使用外部库，user_lib 在 Cargo.toml 中声明，它类似于 stdlib

#[no_mangle]
fn main() -> i32 {
    println!("Hello, world!");
    unsafe {
        llvm_asm!("sret");
    }
    0
}