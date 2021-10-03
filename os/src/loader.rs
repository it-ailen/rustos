use core::mem;

use crate::{
    config::{APP_BASE_ADDRESS, APP_SIZE_LIMIT, KERNEL_STACK_SIZE, MAX_APP_NUM, USER_STACK_SIZE},
    task::TaskContext,
    trap::TrapContext,
};

fn get_base_i(i: usize) -> usize {
    APP_BASE_ADDRESS + i * APP_SIZE_LIMIT
}

pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe {
        // 从 &num_app +1 开始，复制 num_app+1 个元素(包含 app_2_end)
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
    };
    // 清除 icache
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
    }
    for i in 0..num_app {
        let base_i = get_base_i(i);
        // 清空区域
        (base_i..base_i + APP_SIZE_LIMIT)
            .for_each(|addr| unsafe { (addr as usize as *mut u8).write_volatile(0) });

        // 从数据段加载应用到内存指定位置
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };
        dst.copy_from_slice(src);
    }
}

/// 内核栈
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    /// 栈从高往低生长，所以取 data 的最高地址为初始的 SP
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&self, trap_cx: TrapContext, task_cx: TaskContext) -> &mut TaskContext {
        unsafe {
            let trap_cx_ptr = (self.get_sp() - mem::size_of::<TrapContext>()) as *mut TrapContext;
            *trap_cx_ptr = trap_cx;
            let task_cx_ptr =
                (trap_cx_ptr as usize - mem::size_of::<TaskContext>()) as *mut TaskContext;
            *task_cx_ptr = task_cx;
            task_cx_ptr.as_mut().unwrap()
        }
    }
}

/// 用户栈
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/// 内核栈
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

pub fn init_app_cx(app_id: usize) -> &'static TaskContext {
    let i = get_base_i(app_id);
    KERNEL_STACK[app_id].push_context(
        TrapContext::app_init_context(i, USER_STACK[app_id].get_sp()),
        TaskContext::goto_restore(),
    )
}
