use core::fmt::{Debug, Formatter, Result};

use alloc::{sync::Arc, vec::Vec};

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SZ};

/// easy-fs magic
const EFS_MAGIC: u32 = 0x3b800001;

/// 超级块，位于磁盘第一块(编号为0的块)，用于描述磁盘上的数据结构
/// 采用 C 方式排列，不允许 rust 编译器对些结构进行重排，因为它是与磁盘上数据一一对应的
#[repr(C)]
pub struct SuperBlock {
    /// 超级块魔数，用于标识其合法性
    magic: u32,
    /// 文件系统的总块数。这里只是文件系统的总块数，它可能不占用磁盘的所有块。
    pub total_blocks: u32,
    // 后面的四个字段则分别给出 easy-fs 布局中后四个连续区域的长度各为多少个块
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl SuperBlock {
    /// 初始化超级块
    /// 各个区域的块数是以参数的形式传入进来的，它们的划分是更上层的磁盘块管理器需要完成的工作。
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

impl Debug for SuperBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("SuperBlock")
            .field("total_blocks", &self.total_blocks)
            .field("inode_bitmap_blocks", &self.inode_bitmap_blocks)
            .field("inode_area_blocks", &self.inode_area_blocks)
            .field("data_bitmap_blocks", &self.data_bitmap_blocks)
            .field("data_area_blocks", &self.data_area_blocks)
            .finish()
    }
}

/// 此 INode 直接块的数量
const INODE_DIRECT_COUNT: usize = 28;
/// 直接块能存储的数据块数量
const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
/// 一级间接块数量：为一个块的字节数 / 4，即4字节代表一块(usize)
const INODE_INDIRECT1_COUNT: usize = BLOCK_SZ / 4;
/// 二级间接块数量：多个一级间接块组成
const INODE_INDIRECT2_COUNT: usize = INODE_INDIRECT1_COUNT * INODE_INDIRECT1_COUNT;
/// 一级间接块的 ID 范围。(含直接块)
const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_INDIRECT1_COUNT;
/// 二级间接块的 ID 范围。(含一级间接块)
const INDIRECT2_BOUND: usize = INDIRECT1_BOUND + INODE_INDIRECT2_COUNT;

/// 磁盘上块索引结点的类型
#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

/// 每个文件、目录在磁盘上均以 DiskInode 的形式存储，此结构包含它们的元数据
/// 此结构与磁盘上存储结构一致，所以采用C结构方式，避免 rust 重排
#[repr(C)]
pub struct DiskInode {
    /// 文件/目录内容的字节数
    pub size: u32,
    /// direct/indirect1/indirect2 都是存储文件、目录数据的块的索引，故称为 indexNode
    /// 索引分为直接块与间接块，数据写入的优先级为 直接块 > 一级间接块 > 二级间接块
    /// - 直接块直接指向数据，效率高（只有一次查询），但容量有限；
    /// - 间接块可以通过多次指向，定位范围灵活，但多次指向有开销。
    /// 直接块：BLOCK_SIZE * INNODE_DIRECT_COUNT 这么大的容量
    pub direct: [u32; INODE_DIRECT_COUNT],
    /// 1 级间接块：存储 BLOCK_SIZE / 4 * BLOCK_SIZE
    pub indirect1: u32,
    /// 2 级间接块
    pub indirect2: u32,
    /// 文件、目录类型
    type_: DiskInodeType,
}

type IndirectBlock = [u32; BLOCK_SZ / 4];

impl DiskInode {
    /// 初始化目录/文件
    pub fn initialize(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct.iter_mut().for_each(|p| *p = 0);
        self.indirect1 = 0;
        self.indirect2 = 0;
        self.type_ = type_;
    }

    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }

    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }

    /// 根据此 Inode 内部的 id，得到它在整个磁盘上的 block_id
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INODE_INDIRECT1_COUNT {
            get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INODE_INDIRECT1_COUNT;
            let indirect1 = get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect2: &IndirectBlock| indirect2[last]);
            get_block_cache(indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect: &IndirectBlock| {
                    indirect[last % INODE_INDIRECT1_COUNT]
                })
        }
    }

    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SZ as u32 - 1) / BLOCK_SZ as u32
    }

    /// 返回本 Inode 数据所占用的块数量
    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }

    /// 数据量需要的总块数: 数据块 + 1/2级间接块
    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks as usize;
        if data_blocks > INODE_DIRECT_COUNT {
            total += 1; // 一个顶部间接块
        }
        // 2级间接间接块指向多个间接块
        if data_blocks > INODE_INDIRECT1_COUNT {
            total +=
                (data_blocks - INDIRECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;
        }
        total as u32
    }

    /// 需要扩容的块数
    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    /// 文件扩容
    /// new_size: 扩充后的文件大小
    /// new_blocks 是本次分配到的增量磁盘块列表。
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        // 当前已存好的块数
        let mut current_blocks = self.data_blocks();
        self.size = new_size;
        // 所有待存的块数
        let mut total_blocks = self.data_blocks();
        // 分配的块
        let mut new_blocks = new_blocks.into_iter();
        // 先存在直接块中
        while current_blocks < total_blocks.min(INODE_DIRECT_COUNT as u32) {
            self.direct[current_blocks as usize] = new_blocks.next().unwrap();
            current_blocks += 1;
        }
        // 再存到一级间接块中
        if total_blocks > INODE_DIRECT_COUNT as u32 {
            if current_blocks == INODE_DIRECT_COUNT as u32 {
                self.indirect1 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_DIRECT_COUNT as u32; // 在间接块上从0开始计
            total_blocks -= INODE_DIRECT_COUNT as u32; // 直接块已在存了，total_blocks 理解为待存块数
        } else {
            // 直接块够存了
            return;
        }
        // 填充一级间接块
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect: &mut IndirectBlock| {
                while current_blocks < total_blocks.min(INODE_INDIRECT1_COUNT as u32) {
                    indirect[current_blocks as usize] = new_blocks.next().unwrap();
                    current_blocks += 1;
                }
            });
        // 填充二级间接块
        if total_blocks > INODE_INDIRECT1_COUNT as u32 {
            if current_blocks == INODE_INDIRECT1_COUNT as u32 {
                self.indirect2 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_INDIRECT1_COUNT as u32;
            total_blocks -= INODE_INDIRECT1_COUNT as u32;
        } else {
            // 一级间接块就够了
            return;
        }
        // fill indirect2 from (a0, b0) -> (a1, b1)
        let mut a0 = current_blocks as usize / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks as usize % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks as usize / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks as usize % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |block: &mut IndirectBlock| {
                while (a0 < a1) || (a0 == a1 && b0 < b1) {
                    if b0 == 0 {
                        block[a0] = new_blocks.next().unwrap();
                    }
                    get_block_cache(block[a0] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            indirect1[b0] = new_blocks.next().unwrap();
                        });
                    // 下一页
                    b0 += 1;
                    if b0 == INODE_INDIRECT1_COUNT {
                        b0 = 0;
                        a0 += 1;
                    }
                }
            });
    }

    /// 清零，返回待清除的块 ID，由外面负责清除数据内容
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        // 待清除的 block 数量
        let mut data_blocks = self.data_blocks() as usize;
        self.size = 0;
        let mut current_blocks = 0usize;
        // 先清除直接块
        while current_blocks < data_blocks.min(INODE_DIRECT_COUNT) {
            v.push(self.direct[current_blocks]);
            self.direct[current_blocks] = 0;
            current_blocks += 1;
        }
        // 一级间接块
        if data_blocks > INODE_DIRECT_COUNT {
            v.push(self.indirect1);
            data_blocks -= INODE_DIRECT_COUNT;
            current_blocks = 0; // 待清除数和已清除数同时减 INODE_DIRECT_COUNT
        } else {
            return v;
        }
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |block: &mut IndirectBlock| {
                while current_blocks < data_blocks.min(INODE_INDIRECT1_COUNT) {
                    v.push(block[current_blocks]);
                    block[current_blocks] = 0;
                    current_blocks += 1;
                }
            });
        self.indirect1 = 0;
        // 二级间接块
        if data_blocks > INODE_INDIRECT1_COUNT {
            v.push(self.indirect2);
            data_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return v;
        }
        assert!(data_blocks <= INODE_INDIRECT2_COUNT);
        let a1 = data_blocks / INODE_INDIRECT1_COUNT;
        let b1 = data_blocks % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                for i in 0..a1 {
                    v.push(indirect2[i]);
                    get_block_cache(indirect2[i] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for j in 0..INODE_INDIRECT1_COUNT {
                                v.push(indirect1[j]);
                            }
                        });
                }
                if b1 > 0 {
                    // 有未填满的块
                    v.push(indirect2[a1]);
                    get_block_cache(a1 as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for j in 0..b1 {
                                v.push(indirect1[j]);
                            }
                        });
                }
            });
        self.indirect2 = 0;
        v
    }
}

type DataBlock = [u8; BLOCK_SZ];

impl DiskInode {
    /// 在文件(Inode)的 offset 偏移处读取数据 并返回已读字节数
    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_block = offset / BLOCK_SZ;
        let mut read_size = 0usize;
        loop {
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(start_block, Arc::clone(block_device))
                .lock()
                .read(0, |block: &DataBlock| {
                    let src = &block[start % BLOCK_SZ..start % BLOCK_SZ + block_read_size];
                    dst.copy_from_slice(src);
                });
            read_size += block_read_size;
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        read_size
    }

    /// 在文件的 offset 处写数据
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_block = offset / BLOCK_SZ;
        let mut write_size = 0usize;
        loop {
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            let block_write_size = end_current_block - start;
            let src = &buf[write_size..write_size + block_write_size];
            get_block_cache(start_block, Arc::clone(block_device))
                .lock()
                .modify(0, |block: &mut DataBlock| {
                    let dst = &mut block[start % BLOCK_SZ..start % BLOCK_SZ + block_write_size];
                    dst.copy_from_slice(src);
                });
            write_size += block_write_size;
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        write_size
    }
}

/// 文件名长度上限
const NAME_LENGTH_LIMIT: usize = 27;
/// size_of(DirEntry)
pub const DIRENTRY_SZ: usize = 32;

/// 目录项
/// 所有硬盘与内存完全一样的数据都需要用 C 的方式存放
#[repr(C)]
pub struct DirEntry {
    /// 以 0 结尾
    name: [u8; NAME_LENGTH_LIMIT + 1],
    ///
    inode_number: u32,
}

impl DirEntry {
    pub fn new(name: &str, inode_number: u32) -> Self {
        let mut n = [0u8; NAME_LENGTH_LIMIT + 1];
        &mut n[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: n,
            inode_number,
        }
    }

    pub fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_number: 0,
        }
    }

    /// 将当前目录项看作是字节数组
    /// 供 read_at/write_at 使用
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENTRY_SZ) }
    }

    /// 将当前目录项看作是字节数组
    /// 供 read_at/write_at 使用
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENTRY_SZ) }
    }

    pub fn name(&self) -> &str {
        let len = (0usize..).find(|i| self.name[*i] == 0).unwrap();
        core::str::from_utf8(&self.name[..len]).unwrap()
    }

    pub fn inode_number(&self) -> u32 {
        self.inode_number
    }
}
