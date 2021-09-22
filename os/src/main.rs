#![no_std] // 告知编译器不使用 std 库，而使用 core
#![no_main] // 没有一般意义上的main
#![feature(llvm_asm)]

use syscall::sys_exit; //

mod lang_items;
mod syscall;
mod std;

#[no_mangle]
extern "C" fn _start() {
    print!("newline by myself\n");
    println!("Hello, world!");
    sys_exit(9);
    // loop {};
}
