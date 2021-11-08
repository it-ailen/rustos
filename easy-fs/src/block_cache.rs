use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{block_dev::BlockDevice, BLOCK_SZ};

const BLOCK_CACHE_SIZE: usize = 16;

/// 块缓存：读写缓存
pub struct BlockCache {
    /// 数据
    cache: [u8; BLOCK_SZ],
    /// 对应的块 ID
    block_id: usize,
    /// 本块对应的块设备接口，使用 dyn 表示子类型泛型，即在运行时确定类型
    block_device: Arc<dyn BlockDevice>,
    /// 是否被修改，用于 flush
    modified: bool,
}

impl BlockCache {
    /// 根据 block_id 和 device 加载数据，并生成 BlockCache 对象
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SZ];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    /// 根据 offset 得到对应位置的地址
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    /// 获取类型 T 的只读引用，注意：T必须是 Sized，即固定类型
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let a = self.addr_of_offset(offset);
        unsafe { &*(a as *const T) }
    }

    /// 获取类型 T 的可变引用，注意：T必须是 Sized，即固定类型
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let a = self.addr_of_offset(offset);
        unsafe { &mut *(a as *mut T) }
    }

    /// 将修改内容同步回 blockDevice
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }

    /// 获取位于 offset 的类型T的指针 V，并传给 f 用于闭包执行
    /// FnOnce 参考 https://time.geekbang.org/column/article/424009，表示只能调用一次的
    /// 闭包，它只能使用一次的原因是它将自己的内部数据所有权转移给了外面（返回时转移）
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    /// 获取位于 offset 的类型T的指针 V，并传给 f 用于闭包执行
    /// FnOnce 参考 https://time.geekbang.org/column/article/424009，表示只能调用一次的
    /// 闭包，它只能使用一次的原因是它将自己的内部数据所有权转移给了外面（返回时转移）
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

/// 块缓存管理器，维护一个队列，并保证同一时间只有指定的块缓存在内存中
/// 目前采取简单的 FIFO 算法
pub struct BlockCacheManager {
    /// 维护先进先出的块队列
    /// usize: 表示块编号
    /// Arc<Mutex<BlockCache>>: 表示真正的块缓存。通过 Arc<Mutex<...>> 组合，在Manager保留
    /// 一个引用的同时，可以给调用方提供安全的、共享引用和互斥访问，并提供内部可变性。
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            // 如果该 block 已经缓存了，则直接返回就好
            Arc::clone(&pair.1)
        } else {
            if self.queue.len() == BLOCK_CACHE_SIZE {
                // 当前存在在内存中的块缓存数已超出上线，则从列表中从前往后淘汰一块
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                {
                    // find 过滤当前强引用数只有1的块，这表示只有 BlockCacheManager 还持有其引用，可以安全地删除。
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // 加载数据并把缓存放入队列尾部
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    static ref BLOCK_CACHE_MANGER: Mutex<BlockCacheManager> = Mutex::new(BlockCacheManager::new());
}

/// 从全局块缓存管理器中取出缓存
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANGER
        .lock()
        .get_block_cache(block_id, block_device)
}
