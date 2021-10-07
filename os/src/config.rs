pub const USER_STACK_SIZE: usize = 4096 * 2; // 一个用户任务分配 8 KB 空间
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 4;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;


/// 时钟频率，与硬件有关。
// 这儿提供的是 qemu 的配置时钟，可用 cfg 编译开关指定。
// #[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12500000;