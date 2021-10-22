use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use riscv::paging::PTE;
use spin::{Mutex, MutexGuard};

use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    task::pid::pid_alloc,
    trap::{trap_handler, TrapContext},
};

use super::{
    pid::{KernelStack, PidHandle},
    TaskContext,
};

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
    /// 进程退出（调用 exit），但系统没有回收所有资源，这时处于 zombie 状态
    Zombie,
}

pub struct TCBInner {
    /// TaskContext 的指针, 处于任务 KernelStack 的顶部，创建 TCB 时赋值。
    /// 在 __switch 中被修改
    pub task_cx_ptr: usize,
    /// 任务状态
    pub task_status: TaskStatus,
    /// 任务的地址空间
    pub memory_set: MemorySet,
    /// 本任务的 TrapContext 所处的 PPN。
    /// 内核无法通过虚拟地址访问到应用的页面，所以只能用 ppn 来得到数据
    /// ppn 能访问的原因是 trapContext 是使用的恒等映射。
    pub trap_cx_ppn: PhysPageNum,
    /// 在应用地址空间中从  开始到用户栈结束一共包含多少字节，目前不含动态分配的数据
    pub base_size: usize,
    /// 父进程的 TCB 结构
    pub parent: Option<Weak<TCB>>,
    /// 子进程列表的 TCB 结构
    pub children: Vec<Arc<TCB>>,
    /// 退出码
    pub exit_code: i32,
}

impl TCBInner {
    /// 获取本 TCB 表示的 TaskContext 指针的引用；
    // __switch 函数需要这个值作为输入，这说明 __switch 操作的 TaskContext 是处于内核空间的
    pub fn get_task_cx_ptr2(&self) -> *const usize {
        &self.task_cx_ptr as *const usize
    }

    /// 获取地址空间的 token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    /// 获取任务的 TrapContext
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

/// 程序控制块，内核记录任务执行状态的结构
pub struct TCB {
    // 不可变数据放外面
    pub pid: PidHandle,
    /// 任务对应的内核栈
    kernel_stack: KernelStack,
    /// 可变数据
    inner: Mutex<TCBInner>,
}

impl TCB {
    /// 获取内部可变数据。
    pub fn acquire_inner_lock(&self) -> MutexGuard<TCBInner> {
        self.inner.lock()
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    /// 加载一个 elf 到当前执行进程上下文
    pub fn exec(&self, elf_data: &[u8]) {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 继续持有当前 PCB
        let mut inner = self.acquire_inner_lock();
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;
        // 重新初始化 trapCX
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
    }

    /// 从当前任务 fork 一个新任务。
    /// 经过 fork 后，相同的有：
    /// 1. 所有虚拟逻辑段地址
    /// 不同的：
    /// 1. pid 及其对应的 kernelStack
    /// 2. taskContext 位置（放在 KernelStack 栈顶）
    /// 3. 所有 ppn，含 trapContext 所在的 ppn
    pub fn fork(self: &Arc<TCB>) -> Arc<TCB> {
        let mut parent_inner = self.acquire_inner_lock();
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 分配 pid 及 kernelStack
        let pid = pid_alloc();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.get_top();
        let task_cx_ptr = kernel_stack.push_on_top(TaskContext::goto_trap_return());
        let tcb = Arc::new(TCB {
            pid,
            kernel_stack,
            inner: Mutex::new(TCBInner {
                task_cx_ptr: task_cx_ptr as usize,
                task_status: TaskStatus::Ready,
                memory_set,
                trap_cx_ppn,
                base_size: parent_inner.base_size,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
            }),
        });
        parent_inner.children.push(tcb.clone());
        let trap_cx = tcb.acquire_inner_lock().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        tcb
    }

    /// 获取 elf_data(应用镜像入口) 指针，返回新建的程序控制块
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 分配 pid 及内核栈
        let pid = pid_alloc();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.get_top();
        // push a task context which goes to trap_return to the top of kernel stack
        // 将 kernel_stack 的顶部设置为 taskContext，并将其 ra 置为 restore
        let task_cx_ptr = kernel_stack.push_on_top(TaskContext::goto_trap_return());
        let tcb = Self {
            pid,
            kernel_stack,
            inner: Mutex::new(TCBInner {
                task_cx_ptr: task_cx_ptr as usize, // 指向 kernel_stack 的顶部
                task_status: TaskStatus::Ready,
                memory_set,
                trap_cx_ppn,
                base_size: user_sp,
                parent: None,
                children: Vec::new(),
                exit_code: 0,
            }),
        };
        // 初始化用户空间的 TrapContext
        let trap_cx = tcb.acquire_inner_lock().get_trap_cx();
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
