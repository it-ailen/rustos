use core::alloc::Layout;

use buddy_system_allocator::LockedHeap;

use crate::config::KERNEL_HEAP_SIZE;

// LockedHeap 实现了 GlobalAlloc 要求的抽象接口
// alloc 库需要提供一个全局动态内存分配器，用#[global_allocator] 标记（注册）
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// 用于分配的内核堆空间
// 全局初始化数据，链接后被放置于 .bss 段中
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// 初始化内核堆，支持使用动态数据结构。
/// HEAP_SPACE 分配的空间实际是编译器预留的空间（通过数组类型指定），主要用于支持在内核
/// 中进行动态数据分配。
pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

// 使用 #[alloc_error_handler] 宏注册错误处理函数，对应 main.rs 的 feature
#[alloc_error_handler]
pub fn handle_alloc_error(layout: Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
