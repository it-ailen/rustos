use crate::trap::trap_return;



#[repr(C)] // 按 C 语言方式对齐，这样可以在汇编中直接使用，rust 默认会按最省内存的方式重排属性
pub struct TaskContext {
    /// 返回地址
    ra: usize,
    /// s0~s11
    s: [usize; 12],
}

impl TaskContext {
    pub fn goto_restore() -> Self {
        extern "C" { fn __restore(); }
        Self {
            ra: __restore as usize,
            s: [0; 12],
        }
    }

    /// 返回 ra 为 trap_return 的任务上下文。
    /// trap_return 为用户空间的陷入返回函数。
    /// 在地址空间模式下，用户程序发生的任何“trap”返回时都是通过 trap_return 返回到用户空间的,
    /// 所以以 trap_return 为陷入后返回的统一入口。
    pub fn goto_trap_return() -> Self {
        Self {
            ra: trap_return as usize,
            s: [0; 12],
        }
    }
}

