use core::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    char::MAX,
    mem::size_of,
};
use lazy_static::lazy_static;

use crate::{println, trap::TrapContext};

const PAGE_SIZE: usize = 4096;
const USER_STACK_SIZE: usize = PAGE_SIZE * 2; // 用户栈大小
const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2; // 内核栈大小
const MAX_APP_NUM: usize = 16; // 批处理系统最大支持的任务数量
const APP_BASE_ADDRESS: usize = 0x80400000; // 与链接器设置的 user_lib 入口对应
const APP_SIZE_LIMIT: usize = 0x20000; // 应用程序最大范围

// 内核栈
#[repr(align(4096))] // 修改定义的结构体，使其内存对齐为 4096字节
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    /// 返回内核栈初始栈顶
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&self, cx: TrapContext) -> &TrapContext {
        let cx_ptr = (self.get_sp() - size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
            cx_ptr.as_mut().unwrap()
        }
    }
}

// 用户栈结构
#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    fn get_sp(&self) -> usize {
        // 栈从上往下生长，所以初始化的 SP 指向高地址
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

struct AppManager {
    inner: RefCell<AppManagerInner>,
}

unsafe impl Sync for AppManager {}

struct AppManagerInner {
    /// 应用数量
    num_app: usize,
    /// 当前应用
    current_app: usize,
    /// 各任务的入口地址
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManagerInner {
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            panic!("All applications completed!");
        }
        println!("[kernel] Loading app_{}", app_id);
        unsafe {
            // clear icache，此处加载了新的任务，原有缓存的指令已失效，所以需要清空让处理器重新从内存在加载代码
            llvm_asm!("fence.i" :::: "volatile");
            // 清除上一个任务
            (APP_BASE_ADDRESS..APP_BASE_ADDRESS + APP_SIZE_LIMIT).for_each(|addr| {
                (addr as *mut u8).write_volatile(0);
            });
            let app_src = core::slice::from_raw_parts(
                self.app_start[app_id] as *const u8,
                self.app_start[app_id + 1] - self.app_start[app_id],
            );
            let app_dst =
                core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
            app_dst.copy_from_slice(app_src);
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

// lazy_static 宏使静态变量初始化使用懒模式
lazy_static! {
    // ref 与 & 作用一样。但在 static 变量声明中，只能用 ref
    static ref APP_MANAGER: AppManager = AppManager{
        inner: RefCell::new({
            extern "C" {fn _num_app();}// 应用程序向量地址，对应 link_app.S 中的 _num_app 标号
            let num_app_ptr = _num_app as usize as *const usize; // 转成const usize指针，进行只读保护
            let num_app = unsafe {
                // todo 为什么使用 volatile 指令？应该是避免读取指令重排
                num_app_ptr.read_volatile()
            }; // read_volatile 是 unsafe 的，所有它必须放在 unsafe 块中
            let mut app_start: [usize; MAX_APP_NUM+1] = [0; MAX_APP_NUM+1];
            let app_start_raw: &[usize] = unsafe {
                // from_raw_parts 可能访问其它非法位置，所以需要 unsafe
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app+1)
            };
            // 将 app_start_raw 开始的数组复制到 app_start 切片中，要保证它们长度相等
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManagerInner{
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

pub fn run_next_app() -> ! {
    let current_app = APP_MANAGER.inner.borrow().get_current_app();
    APP_MANAGER.inner.borrow().load_app(current_app);
    APP_MANAGER.inner.borrow_mut().move_to_next_app();
    extern "C" {
        fn __restore(cx_addr: usize); // 此声明链接到汇编中的 __restore 标号
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}

pub fn init() {
    print_app_info();
}

pub fn print_app_info() {
    APP_MANAGER.inner.borrow().print_app_info();
}