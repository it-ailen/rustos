use riscv::register::sstatus::{self, Sstatus, SPP};

use super::trap_handler;

/// Trap 时需要保存的执行上下文
pub struct TrapContext {
    /// x0~x31，32个通用寄存器
    pub x: [usize; 32],
    /// sstatus 寄存器
    pub sstatus: Sstatus,
    /// sepc 寄存器，指向 trap 前最后一条指令地址，
    /// 这里实际上是存储的 trap 返回时的下一条指令
    pub sepc: usize,
    // 下面各项由操作系统初始化 TrapContext 时写入，并保持不变，应用
    // 运行不会影响它们的值。放这里是为了方便汇编代码访问
    /// 内核地址空间 satp 值，页表入口
    pub kernel_satp: usize,
    /// 内核栈顶指针的虚拟地址
    pub kernel_sp: usize,
    /// trap handler 入口的虚拟地址
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// 初始化 TrapContext
    /// entry: 要执行的应用程序入口地址
    /// sp: 应用的栈顶
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}
