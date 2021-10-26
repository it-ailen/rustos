mod stdio;
mod pipe;

use crate::mm::UserBuffer;
pub use stdio::*;
pub use pipe::*;

pub trait File: Send + Sync {
    fn read(&self, user_buf: UserBuffer) -> usize;
    fn write(&self, user_buf: UserBuffer) -> usize;
}
