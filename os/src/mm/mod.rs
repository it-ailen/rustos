mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{StepByOne, VPNRange};
pub use page_table::{PTEFlags, PageTable};

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, FrameTracker, frame_dealloc};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::UserBuffer;
pub use page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTableEntry, translated_ref};
pub use memory_set::kernel_token;

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.lock().activate();
}
