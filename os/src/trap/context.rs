use riscv::register::sstatus::{self, Sstatus, SPP};

use super::trap_handler;

/// Trap 时需要保存的执行上下文(用户空间才需要，因为只有U模式下才有陷入，S模式下的陷入
/// 被屏蔽了)
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

    /// 初始化应用空间的 TrapContext。并设置在 trap 返回时 CPU 恢复到 User 模式
    /// entry: 要执行的应用程序入口地址，设置为初始的 sepc，这样在 trap_return 时就能从入口开始运行
    /// sp: 应用的栈顶
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        // set CPU privilege to User after trapping back
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // 初始时，设置 sepc 为程序入口地址。这是由于在此进程被换入运行时，是从 trap 恢复
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }
}
