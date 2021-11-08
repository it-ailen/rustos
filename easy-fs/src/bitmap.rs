use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SZ};

/// 块大小(比特数)
pub const BLOCK_BITS: usize = BLOCK_SZ * 8;

/// 利用连续块表示位图
pub struct Bitmap {
    /// 起始块的 ID
    start_block_id: usize,
    /// 连续的块数
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// 分配一个空闲的位，返回其对应序号(从0开始)
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            // 依次遍历当前 Bitmap 所表示的连续块位图，找出第一个空闲位
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // 把该 block 当作一个 BitmapBlock 看待，并通过闭包函数进行修改
                // 从连续块中分配位图中分配第一个空闲块
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX) // u64::MAX 表示bits 全为1，即全被占用
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                // trailing_ones 返回二进制位中低位尾部的1的个数，即首个0出现的位置
                {
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                } else {
                    // 所有的块被被分配 了
                    None
                }
            });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    /// 使用 usize 表示一个 bit 在 Bitmap 中的位置
    /// 返回:
    /// block_pos: 该位所处的块
    /// bits64_pos: 块内以64位为一组时，所处的组位置
    /// inner_pos: 在64位组中所处的位置
    fn decomposition(mut bit: usize) -> (usize, usize, usize) {
        let block_pos = bit / BLOCK_BITS;
        bit %= BLOCK_BITS;
        (block_pos, bit / 64, bit % 64)
    }

    /// 回收一个块，实际上就是该块对应的位图清零
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bits: usize) {
        let (block_pos, bits64_pos, inner_pos) = Self::decomposition(bits);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }

    /// 本位图的最高位
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}

/// 是一个磁盘数据结构，它将位图区域中的一个磁盘块解释为长度为 64 的一个 u64 数组，
/// 每个 u64 打包了一组 64 bits，于是整个数组包含 64 * 64 = 4096 bits，
/// 且可以以组为单位进行操作，每位代表一个块，1表示占用，0表示可用
type BitmapBlock = [u64; 64];
