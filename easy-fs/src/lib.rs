#![no_std]

/// 块大小（字节数）
pub const BLOCK_SZ: usize = 512;

extern crate alloc;

mod block_dev;
mod block_cache;
mod layout;
mod bitmap;
mod efs;
mod vfs;

pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
pub use vfs::Inode;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
