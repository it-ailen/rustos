use alloc::sync::Arc;
use spin::Mutex;

use crate::{BLOCK_SZ, bitmap::{Bitmap, BLOCK_BITS}, block_cache::get_block_cache, block_dev::BlockDevice, layout::{DiskInode, DiskInodeType, SuperBlock}, vfs::Inode};

/// 文件系统: 负责将逻辑的目录、文件等抽象对应到磁盘上具体的块。
/// 主要分成5部分连续空间：
/// - 超级块：占磁盘第一个块，提供合法检测（魔数），描述磁盘整体布局，如总空间大小，inode 数量，数据块数量等
/// - inode Bitmap：inode 位图区，长度为若干个块，一位代表一个 inode 的使用情况
/// - inode area：inode 区域，长度为若干个块，存一个个 inode 结构，与 inode bitmap 一一对应
/// - data Bitmap：数据位图区，长度为若干个块，1位代表一个数据块的使用情况
/// - data block area：数据块区域，长度为若干个块，1位代表一个数据块的使用情况
pub struct EasyFileSystem {
    /// 此文件系统所属块设备
    pub block_device: Arc<dyn BlockDevice>,
    /// 磁盘的第二部分，inode 位图区域，描述 inode 的使用情况
    pub inode_bitmap: Bitmap,
    /// 磁盘的第4部分，数据块位图区域，描述数据块的使用情况
    pub data_bitmap: Bitmap,
    /// 磁盘的第3部分，存放 inode 数据的块区域
    inode_area_start_block: u32,
    /// 磁盘的第5部分，存放数据的区域
    data_area_start_block: u32,
}

type DataBlock = [u8; BLOCK_SZ];

impl EasyFileSystem {
    /// 初始化一个 EFS 对象。初始化超级块、inode区域、数据区域，以级根目录。
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // 从第2个（序号1）块开始
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        // 位图能表示的 inode 数量
        let inode_num = inode_bitmap.maximum();
        // inode 占用的块数
        let inode_area_blocks =
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
        // 1 为超级块所占的块
        let data_total_blocks = total_blocks - inode_total_blocks - 1;
        let data_bitmap_blocks = (data_total_blocks + BLOCK_BITS as u32) / (BLOCK_BITS as u32 + 1);
        // data_bitmap 位于 inode 之后
        let data_bitmap = Bitmap::new(
            (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        // let data_area_blocks = data_bitmap.maximum();
        // 有几个块会多？
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // 清除所有块
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |block: &mut DataBlock| {
                    for byte in block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // 初始化超级块
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .modify(0, |sb: &mut SuperBlock| {
                sb.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            });
        // 分配一个根目录
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |node: &mut DiskInode| {
                node.initialize(DiskInodeType::Directory);
            });
        Arc::new(Mutex::new(efs))
    }

    /// 分配一个 inode 位
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// 分配一个数据块，返回其所在 block_id
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    /// 回收块
    pub fn dealloc_data(&mut self, block_id: u32) {
        // 清除数据（没必要）
        // get_block_cache(
        //     block_id as usize,
        //     Arc::clone(&self.block_device)
        // )
        // .lock()
        // .modify(0, |data_block: &mut DataBlock| {
        //     data_block.iter_mut().for_each(|p| { *p = 0; })
        // });
        let index = block_id - self.data_area_start_block;
        self.data_bitmap.dealloc(&self.block_device, index as usize);
    }

    /// 返回 inode 在磁盘上的位置 (block_id, offset_in_block_by_bytes)
    pub fn get_disk_inode_pos(&self, id: u32) -> (u32, usize) {
        let inode_sz = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_sz) as u32;
        let block_id = self.inode_area_start_block + (id + inodes_per_block - 1) / inodes_per_block;
        (block_id, (id % inodes_per_block) as usize * inode_sz)
    }

    /// 从现存磁盘中打开一个初始化的文件系统
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |sb: &SuperBlock| {
                assert!(sb.is_valid(), "Error loading EFS!");
                let inode_total_blocks = sb.inode_bitmap_blocks + sb.inode_area_blocks;
                let efs = Self {
                    block_device: Arc::clone(&block_device),
                    inode_bitmap: Bitmap::new(1, sb.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        sb.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + sb.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + sb.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }

    /// 读取 efs 上的根目录 inode(inode 编号为0)
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }
}
