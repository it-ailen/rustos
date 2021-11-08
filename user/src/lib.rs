#![no_std]
#![feature(linkage)]
#![feature(llvm_asm)]
#![feature(panic_info_message)] // 使用 panic message，有这个 panic_handler 才能起作用
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;

extern crate alloc;

use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;
use syscall::*;

/// 用户堆空间大小
const USER_HEAP_SIZE: usize = 16384;

/// 使用编译好的数组作为用户空间的堆
static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

/// 全局分配器，内核和用户空间各一个。有了这个分配器，内核才可以使用动态数据类型
#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

/// 必须要指定全局分配出错时的 handler
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

mod syscall;
#[macro_use] // 使 console 中定义的宏能被此 crate 外使用，比如 bin 下面的各应用
pub mod console;
mod lang_items;

#[no_mangle] // 不允许编译器混淆
#[link_section = ".text.entry"] // 告知编译器将此函数放到 .text.entry 节，成为用户程序入口
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    // 操作系统负责初始化用户程序的 .bss 区间
    // clear_bss(); // 系统还不具有清零 .bss 的能力，需要应用程序自己做
    unsafe {
        HEAP.lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    println!("user lib run now!!!");
    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe {
                // 字符串以 \0 结尾
                ((str_start + *i) as *const u8).read_volatile() == 0
            })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe { core::slice::from_raw_parts(str_start as _, len) })
                .unwrap(),
        )
    }
    exit(main(argc, v.as_slice())); // 调用用户库的 exit 方法
    panic!("unreachable after sys_exit!");
}

// 将 main 标记为弱连接，在bin和 lib 下都有 main，但由于lib是弱连接，它会被链接器忽略，替换为没有标记的（强连接）
// 类似于 default，只是为了让 lib 在没有 bin 的时候也能编译通过
#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
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

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

/// 等待任意子进程退出
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code) {
            -2 => {
                // 如果返回 -2， 表示有子进程但进程未结束
                yield_(); // 主动释放 CPU
            }
            exit_pid => return exit_pid, // 结束的子进程 ID
        }
    }
}

/// 等待指定子进程退出。
pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code) {
            -2 => {
                sys_yield();
            }
            exit_pid => return exit_pid,
        }
    }
}

pub fn sleep(period_ms: usize) {
    let start = sys_get_time();
    while sys_get_time() < start + period_ms as isize {
        sys_yield();
    }
}

/// fork 一个新进程。返回 0 表示子进程，>0 表示父进程。
pub fn fork() -> isize {
    sys_fork()
}

/// 从文件中读取数据
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}

pub fn getpid() -> isize {
    sys_getpid()
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}
