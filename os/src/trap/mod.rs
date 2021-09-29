mod context;

pub use context::TrapContext;

use riscv::register::{scause::{self, Exception, Trap}, stval, stvec, utvec::TrapMode};

use crate::syscall::syscall;

use crate::{batch::run_next_app, println, syscall};

global_asm!(include_str!("trap.S"));

/// cx 传入与返回值一样， trap_handler 需要保证它的值不发生变化
// 由汇编代码调用
#[no_mangle] // 不允许编译器进行混淆，因为它要被汇编直接访问，类似于 extern "C"
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // 来自 U 特权级的 environment call(ecall)，即系统调用
            cx.spec += 4; // spec 在 trap 时，会被修改为 trap 前的最后一条指令，这里+4是让它指向下一条指令
            // a0 = syscall(a7, a0, a1, a2)，系统调用规定的寄存器
            println!("[kernel] UserEnv Call now. call({}, {}, {}, {})", cx.x[17], cx.x[10], cx.x[11], cx.x[12]);
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, core dumped.");
            run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, core dumped.");
            run_next_app();
        }
        _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    cx
}

pub fn init() {
    extern "C" { fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}
