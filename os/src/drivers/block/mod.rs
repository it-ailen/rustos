use alloc::sync::Arc;
use easy_fs::BlockDevice;
// use self::virtio_blk::VirtIOBlock;
use lazy_static::*;

mod virtio_blk;

// #[cfg(feature = "board_qemu")]
type BlockDeviceImpl = virtio_blk::VirtIOBlock;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}


