mod context;

use crate::{config::{TRAMPOLINE, TRAP_CONTEXT}, syscall::syscall, task::{current_trap_cx, current_user_token, exit_current_and_run_next, suspend_current_and_run_next}, timer::set_next_trigger};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// 设置用户程序陷入时的处理函数(统一到跳板地址)
fn set_user_trap_entry() {
    // 跳板地址实际上就是 __alltraps 的地址
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

/// 设置内核陷入时的处理函数
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

// 由汇编代码调用
#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_cx();
            // 来自 U 特权级的 environment call(ecall)，即系统调用
            cx.sepc += 4; // spec 在 trap 时，会被修改为 trap 前的最后一条指令，这里+4是让它指向下一条指令
                          // a0 = syscall(a7, a0, a1, a2)，系统调用规定的寄存器
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]);
            // 经过可能的 sys_exec 后，当前任务已发生变化，所以需要重新加载 cx
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault) | 
        Trap::Exception(Exception::StorePageFault) |
        Trap::Exception(Exception::InstructionFault) |
        Trap::Exception(Exception::InstructionPageFault) |
        Trap::Exception(Exception::LoadFault) |
        Trap::Exception(Exception::LoadPageFault) => {
            println!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
            );;
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, core dumped.");
            exit_current_and_run_next(-3);
            //run_next_app();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}

/// 陷入完成后的返回函数
#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    // 获取当前任务的页表入口
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        // 清除 icache，这里指定 volatile 使编译器不对此指令进行重排
        llvm_asm!("fence.i" :::: "volatile");
        // 调用 __restore(a0, a1)
        llvm_asm!("jr $0" :: "r"(restore_va), "{a0}"(trap_cx_ptr), "{a1}"(user_satp) :: "volatile");
    }
    panic!("Unreachable in back_to_user")
}

/// 此时已处理 S 模式，再次 Trap 的功能暂时不实现
#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!("trap from kernel");
}

pub use context::TrapContext;
