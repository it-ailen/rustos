use riscv::register::sstatus::{self, Sstatus, SPP};

/// Trap 时需要保存的执行上下文
pub struct TrapContext {
    /// x0~x31，32个通用寄存器
    pub x: [usize; 32],
    /// sstatus 寄存器
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// entry 为当前要执行的应用程序入口地址（记住这是批处理系统）
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }
}
