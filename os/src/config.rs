pub const USER_STACK_SIZE: usize = 4096 * 2; // 一个用户任务分配 8 KB 空间
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 4;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// 跳板位置，处于64位空间顶部1页的位置
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// TrapContext 所处的虚拟页，放在次高页
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

/// 内存页大小
pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

/// 内核堆大小
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
/// 物理内存上限，后面应该使用设备查询获取
pub const MEMORY_END: usize = 0x80800000;


/// 时钟频率，与硬件有关。
// 这儿提供的是 qemu 的配置时钟，可用 cfg 编译开关指定。
// #[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12500000;


/// 返回内核栈区间，(bottom, top)
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    // PAGE_SIZE 是 bottom 上的一个间隔页，避免用户栈超过空间
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}