

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
}

