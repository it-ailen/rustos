use alloc::string::String;
use alloc::vec;
use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use riscv::paging::PTE;
use spin::{Mutex, MutexGuard};

use crate::fs::{File, Stdin, Stdout};
use crate::mm::translated_refmut;
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

    // 资源相关
    /// 文件描述符表，进程打开的文件的描述符列表。
    //
    // Vec：表为动态长度，即固定文件数限制
    // Option: 可利用 None 标志文件描述符是否在使用
    // Arc: 提供并发共享能力，可被多线程同时使用；内容放在堆上，可不在编译期确定大小
    // dyn: 表示运行时多态，即在运行时才知道是什么类型
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
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

    /// 在当前进程文件描述符表中分配一个空闲的文件描述符
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|&fd| self.fd_table[fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
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
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        let (memory_set, mut user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 将参数的指针放到用户栈上
        // 参考 https://rcore-os.github.io/rCore-Tutorial-Book-v3/chapter7/4cmdargs-and-redirection.html#sys-exec
        let usize_len = core::mem::size_of::<usize>();
        // 将 args 存到用户栈的顶部，以0结束；所以这里分配比实际大小多一个空间，用于放0
        user_sp -= (args.len() + 1) * usize_len;
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    memory_set.token(),
                    (argv_base + arg * usize_len) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0; // 以0表示参数结束
        for i in 0..args.len() {
            // 将参数数据复制到 user_sp 参数后的部分
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp; // 参数指针位置，即入参的参数、数据都在栈上
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(memory_set.token(), p as *mut u8) = *c;
                p += 1;
            }
            // 以 \0 结尾
            *translated_refmut(memory_set.token(), p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B for k210 platform
        // 按 4字节对齐 
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // 继续持有当前 PCB
        let mut inner = self.acquire_inner_lock();
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;

        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *inner.get_trap_cx() = trap_cx;
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
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
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
                fd_table: new_fd_table,
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
                fd_table: vec![
                    // 标准输入 0
                    Some(Arc::new(Stdin)),
                    // 标准输出 1
                    Some(Arc::new(Stdout)),
                    // 错误输出 2
                    Some(Arc::new(Stdout)),
                ],
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
