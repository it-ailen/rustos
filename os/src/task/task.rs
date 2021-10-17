use riscv::paging::PTE;

use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    trap::{trap_handler, TrapContext},
};

use super::TaskContext;

// #[derive(...)] 提供一些 trait 的默认实现
// PartialEq 是实现 == 运算符重载的默认方式
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// 未初始化
    UnInit,
    /// 准备运行
    Ready,
    ///
    Running,
    ///
    Exited,
}

/// 程序控制块，内核记录任务执行状态的结构
pub struct TCB {
    /// TaskContext 的指针
    pub task_cx_ptr: usize,
    /// 任务状态
    pub task_status: TaskStatus,
    /// 任务的地址空间
    pub mem_set: MemorySet,
    /// 本任务的 TrapContext 所处的 PPN。
    /// 内核无法通过虚拟地址访问到应用的页面，所以只能用 ppn 来得到数据
    pub trap_cx_ppn: PhysPageNum,
    /// 在应用地址空间中从  开始到用户栈结束一共包含多少字节，目前不含动态分配的数据
    pub base_size: usize,
}

impl TCB {
    /// 获取本 TCB 表示的 TaskContext 指针的引用；
    // __switch 函数需要这个值作为输入，这说明 __switch 操作的 TaskContext 是处于内核空间的
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }

    /// 获取地址空间的 token
    pub fn get_user_token(&self) -> usize {
        self.mem_set.token()
    }

    /// 获取任务的 TrapContext
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    /// 获取 elf_data(应用镜像入口) 指针，返回新建的程序控制块
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        println!("TCB.new app_id={:?}", app_id);
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        // 内核栈通过动态分配
        println!(
            "insert framed area into kernel space({:#x}, {:#x})",
            kernel_stack_bottom, kernel_stack_top
        );
        KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        println!("init taskContext");
        let task_cx_ptr =
            (kernel_stack_top - core::mem::size_of::<TaskContext>()) as *mut TaskContext;
        println!("task_cx_ptr: {:p}", task_cx_ptr);
        unsafe {
            *task_cx_ptr = TaskContext::goto_trap_return();
        }
        println!("build tcb");
        let tcb = Self {
            task_cx_ptr: task_cx_ptr as usize,
            task_status,
            mem_set: memory_set,
            trap_cx_ppn,
            base_size: user_sp,
        };
        let trap_cx = tcb.get_trap_cx();
        println!("Got TrapContext: {:p}", trap_cx);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        println!("Got tcb");
        tcb
    }
}
