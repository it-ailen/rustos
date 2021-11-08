mod stdio;
mod pipe;
mod inode;

use crate::mm::UserBuffer;
pub use stdio::*;
pub use pipe::*;
pub use inode::{open_file, OpenFlags, list_apps};

pub trait File: Send + Sync {
    fn read(&self, user_buf: UserBuffer) -> usize;
    fn write(&self, user_buf: UserBuffer) -> usize;
}
