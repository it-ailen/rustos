use core::fmt::Debug;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use riscv::register::satp;
use spin::Mutex;

use crate::{config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE}, mm::address::StepByOne};

use super::{frame_alloc, PTEFlags, PageTableEntry, PhysPageNum};
use lazy_static::lazy_static;

use super::{
    address::{PhysAddr, VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::FrameTracker,
    page_table::PageTable,
};

// 由 linker 指定，定义内核镜像的符号
extern "C" {
    /// 代码段开始
    fn stext();
    /// 代码段结束
    fn etext();
    /// 只读数据段
    fn srodata();
    fn erodata();

    /// 数据段
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();

    /// kernel 结束，后面内存可供应用使用
    fn ekernel();
    /// 跳板
    fn strampoline();
}

// lazy_static 保证初始化在第一次使用时进行，但其空间在编译期决定。
// 使用 Arc 通过编译器检查，同时在多核环境下会有用。
// 既需要 Arc<T> 提供的共享 引用，也需要 Mutex<T> 提供的互斥访问
lazy_static! {
    /// 内核地址空间：处于
    pub static ref KERNEL_SPACE: Arc<Mutex<MemorySet>> =
        Arc::new(Mutex::new(MemorySet::new_kernel()));
}

/// 地址空间：描述一个任务的内存分配情况
/// 由一系列有关联（同属于一个任务）的逻辑段组成
pub struct MemorySet {
    /// 任务占用的页表节点
    page_table: PageTable,
    /// 已映射的逻辑连续段
    areas: Vec<MapArea>,
}

impl MemorySet {
    ///
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// 返回页表对应的 token
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// 通过 vpn 查找其对应的页表项
    /// *注意*：只返回已映射的。
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 插入一个 area，使用页方式映射
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    /// 映射跳板
    /// 位于虚拟地址空间最高位的最后一页，它被映射到物理空间 kernel 镜像的最后一部分
    /// *注意*：跳板不在地址逻辑段中。
    fn map_trampoline(&mut self) {
        println!("map_trampoline ...");
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// 增加一个逻辑段，并使用 data 对其进行初始化。
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// 返回 kernel 的地址空间（不含内核栈）
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();
        // map kernel sections
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        println!("mapping .text section");
        // 将内核代码段映射为 RX 逻辑段，采用恒等映射，这样保证内核代码在分页开启前后地址不变
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        println!("mapping .rodata section");
        // 只读段
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        println!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::W | MapPermission::R,
            ),
            None,
        );
        println!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("mapping physical memory");
        // 将 ekernel 到物理内存结束的部分设置为可分配内存
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        println!("Kernel set done");
        memory_set
    }

    /// 根据 elf 文件解析出对应的地址空间
    /// 返回
    /// - 应用的地址空间
    /// - 用户栈顶: 向下生长，最大尺寸为 USER_STACK_SIZE
    /// - 应用程序入口
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        assert_eq!(
            elf_header.pt1.magic,
            [0x7f, 0x45, 0x4c, 0x46],
            "invalid elf"
        );
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        // 遍历 program header 数组
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                // 用户空间程序，借助硬件检查
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                let map_area = MapArea::new(
                    start_va,
                    end_va,
                    MapType::Framed, // 用户空间的都不使用恒等映射
                    map_perm,
                );
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        // 映射用户栈
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // 每个用户栈底部（栈向下生长）插入一个空虚拟页，用于守护边界，
        // 避免访问到其它应用的数据。硬件会对地址进行检查，这些空页不会存数据。
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // 映射 TrapContext
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    /// 启动地址空间（页表）
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            // 写 satp 寄存器
            satp::write(satp);
            // 刷新 TLB（快表） 缓存
            llvm_asm!("sfence.vma" :::: "volatile");
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MapType {
    /// 恒等映射，一般用于内核逻辑段，虚拟页号==物理页号，
    /// 主要保证启动分页前后，页地址保持一致
    Identical,
    /// 按页映射，涉及到动态映射
    Framed,
}

bitflags! {
    /// PTEFlags 的一个子集
    pub struct MapPermission: u8 {
        /// read
        const R = 1 << 1;
        /// write
        const W = 1 << 2;
        /// execute
        const X = 1 << 3;
        /// UserSpace 可用
        const U = 1 << 4;
    }
}

/// 逻辑段：描述一段连续的虚拟内存，它可能映射到不连续的物理页。这些连续的虚拟页以同
/// 样的方式映射到物理页
pub struct MapArea {
    /// 虚拟页范围
    vpn_range: VPNRange,

    /// 本逻辑段内已分配的虚拟页 -> 物理页的 map
    /// 只在 Framed 方式时有效
    data_frame: BTreeMap<VirtPageNum, FrameTracker>,

    /// 整个虚拟逻辑段的映射方式，各页面间保持一致
    map_type: MapType,

    /// 本逻辑段映射的权限
    map_perm: MapPermission,
}

impl Debug for MapArea {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapArea").field("vpn_range", &self.vpn_range).field("data_frame", &self.data_frame).field("map_type", &self.map_type).field("map_perm", &self.map_perm).finish()
    }
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn = start_va.floor();
        let end_vpn = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frame: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    /// 将 data 中的数据拷贝到 MapArea 中，且利用 page_table 查询本逻辑段实际的物理页
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)]; // 源数据不超过一页
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()]; // 只取 src 长这么多数据
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                // 已拷贝完
                break;
            }
            // 一页页地拷贝
            current_vpn.step();
        }
    }

    /// 映射一页虚拟页：会根据本 MapArea 的映射类型，确定虚拟页映射到物理页的方法。
    /// 如果是恒等映射，则直接将虚拟页号与物理页号相等即可；
    /// 如果是 framed ，则从 [ekernel, MEMEORY_END) 区间内分配一页
    /// 并前 vpn:ppn 的关系写到 pte 中
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frame.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    /// 去除 vpn 的映射，包括数据页和页表项
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frame.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }

    /// 将本逻辑段的连续虚拟页映射到页表中
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    /// 取消本逻辑段的页映射
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.lock();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space.page_table.translate(mid_text.floor()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_rodata.floor()).unwrap().writable(),
        false,
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_data.floor()).unwrap().executable(),
        false,
    );
    println!("remap_test passed!");
}