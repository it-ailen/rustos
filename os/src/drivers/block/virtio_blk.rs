use alloc::vec::Vec;
use easy_fs::BlockDevice;
use lazy_static::*;
use spin::Mutex;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

use crate::mm::StepByOne;
use crate::mm::{
    frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
    VirtAddr,
};

/// 通过 MMIO 访问VirtIO 设备对应的寄存器组地址。在 config 中定义
const VIRTIO0: usize = 0x10001000;

/// 这里只是将 virtio_drivers crate 的 Blk 加了一个互斥锁，并实现了我们定义的 BlockDevice crate。
/// 驱动细节在此未涉及，由现成的 crate 完成
pub struct VirtIOBlock(Mutex<VirtIOBlk<'static>>);

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

impl VirtIOBlock {
    pub fn new() -> Self {
        Self(Mutex::new(
            VirtIOBlk::new(unsafe {
                // VirtIOHeader 实际上就代表以 MMIO 方式访问 VirtIO 设备所需的一组设备寄存器
                &mut *(VIRTIO0 as *mut VirtIOHeader)
            })
            .unwrap(),
        ))
    }
}

lazy_static! {
    // 将分配的环形队列页页存放在全局变量中，避免被提前回收
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
}

/*
 VirtIO 架构下，需要在公共区域中放置一种叫做 VirtQueue 的环形队列，CPU 可以
 向此环形队列中向 VirtIO 设备提交请求，也可以从队列中取得请求的结果。
 对于 VirtQueue 的使用涉及到物理内存的分配和回收，但这并不在 VirtIO 驱
 动 virtio-drivers 的职责范围之内，因此它声明了数个相关的接口，需要库的使用者自己来实现
// https://github.com/rcore-os/virtio-drivers/blob/master/src/hal.rs#L57

extern "C" {
    fn virtio_dma_alloc(pages: usize) -> PhysAddr;
    fn virtio_dma_dealloc(paddr: PhysAddr, pages: usize) -> i32;
    fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr;
    fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr;
}
*/

#[no_mangle] // 对应 extern "C"
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.lock().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PageTable::from_token(kernel_token())
        .translate_va(vaddr)
        .unwrap()
}
