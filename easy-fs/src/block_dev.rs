use core::any::Any;


/// 块设备操作接口
/// 作为块设备的驱动层，向上隐藏设备读写细节
/// 块与扇区：扇区是块设备随机读写的单位，一般块较扇区更大（比如 4096 vs. 512）
pub trait BlockDevice : Send + Sync + Any {
    /// 根据 block_id 从读取数据
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    /// 往块中写数据
    fn write_block(&self, block_id: usize, buf: &[u8]);
}