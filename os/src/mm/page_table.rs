use alloc::vec::Vec;
use alloc::{string::String, vec};
use bitflags::*;

use super::PhysAddr;
use super::{
    address::{PhysPageNum, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    StepByOne, VirtAddr,
};

bitflags! {
    /// 页表项标志
    /// 页表项组成参考：https://rcore-os.github.io/rCore-Tutorial-Book-v3/chapter4/3sv39-implementation-1.html#id7
    pub struct PTEFlags: u8 {
        /// valid，表示当前 PTE 是否有效，无效说明未映射
        const V = 1 << 0;
        /// 是否可读
        const R = 1 << 1;
        /// 是否可写
        const W = 1 << 2;
        /// 是否可执行
        const X = 1 << 3;
        /// 是否可被用户空间访问
        const U = 1 << 4;
        ///
        const G = 1 << 5;
        /// Accessed，是否被 CPU 访问过。此项会在页分配时被清零，访问过再
        /// 由 CPU 设置，一般用于 swap 统计
        const A = 1 << 6;
        /// Dirty，数据是否被修改。会影响 flush
        const D = 1 << 7;
    }
}

/// 页表项，一项8字节，主要有两部分组成：
/// 0~7：PTE flags
/// 10~53：44位物理页号
// 参考：https://rcore-os.github.io/rCore-Tutorial-Book-v3/chapter4/3sv39-implementation-1.html#id5
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    /// 获取物理页号，10~53 共 44 位
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    /// 获取 PTEFlags，0~7 共 8 位
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.flags() & PTEFlags::V != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
}

/// 任务（进程）分配的页表，表示当前任务用户空间分配到的内存页
pub struct PageTable {
    /// 根页目录页对应的物理页号
    root_ppn: PhysPageNum,
    /// 已分配的物理页，使用 FrameTracker 完成初始化及回收
    frames: Vec<FrameTracker>,
}

impl PageTable {
    /// 分配一个空的页表，完成必要的初始化
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 根据 satp 生成页表
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// 页表 token，用于填充 satp 寄存器
    /// token 的 60~63 位会被置为8，即启动分页模式
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }

    /// 查找或者新建 PTE
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idx = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        // 依次遍历 3 级页表
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idx[i]];
            if i == 2 {
                // L2 是叶子，指向实际的数据页
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                // 页表项未被分配，则分配一个
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame); // 记录已分配的页桢，后续用于释放
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 查找 pte，不负责页表项的分配。即如果页表项未分配过，则返回 None
    /// 从内核视角查找，利用恒等映射完成。
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&PageTableEntry> = None;
        for i in 0..3 {
            let pte = &ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    /// 在任务页表中建立虚拟页号到物理页号间的映射(最终会被 MMU 消费)
    /// 这里的 ppn 是最终数据页，页表组织树的中间结点在 find_pte_create 中完成
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        // println!("PageTable.map: {:?} => {:?}, flags: {:?}", vpn, ppn, flags);
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// 解映射，抹掉对应的 pte
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// 转换虚拟页号对应的页表项。
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| pte.clone())
    }

    /// 通过当前页表，翻译虚拟地址到物理地址的映射(注意：是从内核视角，走恒等映射得到数据)
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
}

/// 将应用地址空间中一个缓冲区转化为在内核空间中能够直接访问的形式
/// token: 页表 token
/// ptr: 应用虚拟地址起点
/// len: buffer 长度
///
/// return: 含可访问区域的页列表
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// 用户空间数据缓冲区; 位于应用地址空间。
pub struct UserBuffer {
    /// 数据缓冲区
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }

    /// 链表中所有片段数据的总长
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;

    type IntoIter = UserBufferIterator;

    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}

/// 通过 token 指向的地址空间页表，读取 ptr 所指向的字符串
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            // 以 \0 结尾
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

/// 通过 token 指向的地址空间页表，读取 ptr 所指向的T指针
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}
