use alloc::{string::String, sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode, DiskInodeType, DIRENTRY_SZ},
};
/// vfs 为文件系统的虚拟接口，实际的实现由具体的文件系统完成。

/// 与 DiskInode 对应。对上层的抽象，调用者不需要知道在块设备中文件的具体存储情况。
/// DiskInode 是硬盘中文件数据，Inode 是内存中的抽象概念。
pub struct Inode {
    /// Inode 所在的块
    block_id: usize,
    /// 块内偏移（字节）
    block_offset: usize,
    /// 所属文件系统, Inode 的所有操作都是由底层文件系统支持的
    fs: Arc<Mutex<EasyFileSystem>>,
    /// 所在设备
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    /// 在当前目录中找文件，返回其 Inode
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_node(|disk_node| {
            self.find_inode_id(name, disk_node).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    /// list 当前目录文件
    pub fn ls(&self) -> Vec<String> {
        /// _ 开头是为了避免作用域内未使用变量而被编译器阻止。
        let _fs = self.fs.lock();
        self.read_disk_node(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENTRY_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENTRY_SZ, dirent.as_bytes_mut(), &self.block_device),
                    DIRENTRY_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    /// 在当前目录中创建文件。目前不支持创建子目录
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self
            .modify_disk_node(|inode| {
                assert!(inode.is_dir());
                self.find_inode_id(name, inode)
            })
            .is_some()
        {
            return None;
        }
        // create a new file
        let new_node_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, block_offset) = fs.get_disk_inode_pos(new_node_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, |inode: &mut DiskInode| {
                inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_node(|inode| {
            let file_count = (inode.size as usize) / DIRENTRY_SZ;
            let new_size = (file_count + 1) * DIRENTRY_SZ;
            self.increase_size(new_size as u32, inode, &mut fs);
            // 写目录项
            let dirent = DirEntry::new(name, new_node_id);
            inode.write_at(
                file_count * DIRENTRY_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        let (block_id, block_offset) = fs.get_disk_inode_pos(new_node_id);
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// 清空目录或者文件
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_node(|disk_node| {
            let size = disk_node.size;
            let data_blocks_dealloc = disk_node.clear_size(&self.block_device);
            assert_eq!(data_blocks_dealloc.len(), DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
    }

    /// 从 offset 处读取数据到 buf 中
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_node(|disk_node| disk_node.read_at(offset, buf, &self.block_device))
    }

    /// 在 offset 处写入数据
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_node(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }
}

impl Inode {
    /// 从硬盘读取 inode 并返回为内存对象
    fn read_disk_node<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    ///
    fn modify_disk_node<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    /// 在文件夹 inode 中查询文件(name) 所对应的 inode id(即 offset)
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENTRY_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(i * DIRENTRY_SZ, dirent.as_bytes_mut(), &self.block_device),
                DIRENTRY_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }

    /// 增加当前 inode 的大小
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        // 还有空间，不需要扩容
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        // 分配需要扩充的块
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(blocks_needed, v, &self.block_device);
    }
}
