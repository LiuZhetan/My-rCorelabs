mod pipe;
mod stdio;
mod inode;

use easy_fs::OSStat;
use crate::mm::UserBuffer;

pub trait File : Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
    // 增加一个方法seek,用于修改OSInode读写指针的位置
    fn seek(&self, _offset:usize, _whence:usize) -> isize {
        -1
    }
    fn stat(&self) -> Option<OSStat> {
        None
    }
}

pub use pipe::{Pipe, make_pipe};
pub use stdio::{Stdin, Stdout};
pub use inode::{OSInode, open_file, link_file, unlink, OpenFlags, list_apps, create_dir, Directory, Fd, OSDirent};