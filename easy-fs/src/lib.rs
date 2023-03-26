#![no_std]

extern crate alloc;

mod block_dev;
mod layout;
mod efs;
mod bitmap;
mod vfs;
mod block_cache;

pub const BLOCK_SZ: usize = 512;
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
pub use vfs::Inode;
use layout::*;
use bitmap::Bitmap;
use block_cache::{get_block_cache, block_cache_sync_all};
// 增加目录项的大小
pub use layout::{DIRENT_SZ,DirEntry};
pub use vfs::{OSStat,FileType};