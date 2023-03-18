use easy_fs::{
    EasyFileSystem,
    Inode,
};
use crate::drivers::BLOCK_DEVICE;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use lazy_static::*;
use bitflags::*;
use alloc::vec::Vec;
use super::File;
use crate::mm::UserBuffer;

pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}

pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

impl OSInode {
    pub fn new(
        readable: bool,
        writable: bool,
        inode: Arc<Inode>,
    ) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner {
                offset: 0,
                inode,
            })},
        }
    }
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

// 对路径检查，得到不含"./"的路径，如果含有“../”返回false
#[inline]
fn check_path(path: &mut Vec<&str>) -> bool {
    if path[0] == "." {
        path.remove(0);
    }
    if path[0] == ".." {
        return false;
    }
    true
}

// dir_path为上层目录数组
fn find_inode(inode: Arc<Inode>, dir_path: &[&str]) -> Option<Arc<Inode>> {
    assert!(dir_path.len() >= 1);
    let next_dir = inode.find(dir_path[0]);
    match next_dir {
        Some(dir) => {
            if dir_path.len() == 1 {
                Some(dir)
            }
            else {
                find_inode(dir, &dir_path[1..])
            }
        },
        None => None
    }
}

fn split_path(path: &str) -> Option<(Arc<Inode>,&str)>{
    let mut path:Vec<_> = path.split('/').collect();
    if !check_path(&mut path) {
        return None;
    }
    let parent_dir =
        if path.len() == 1 {
            // 就在根目录下
            Some(ROOT_INODE.clone())
        }
        else {
            find_inode(
                ROOT_INODE.clone(),
                &path.as_slice()[..(path.len() - 2)]
            )
        };
    match parent_dir {
        Some(inode) => Some((inode, path.last().unwrap())),
        None => None
    }
}

// 修改，使得可以在多级目录下打开或创建文件
pub fn open_file(path: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    let parent_dir;
    let name;
    let res = split_path(path);
    match res {
        Some((inode,file_name)) => {
            parent_dir = inode;
            name = file_name;
        }
        None => {
            return None;
        }
    }

    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = parent_dir.find(name) {
            // clear size
            inode.clear();
            Some(Arc::new(OSInode::new(
                readable,
                writable,
                inode,
            )))
        } else {
            // create file
            parent_dir.create(name)
                .map(|inode| {
                    Arc::new(OSInode::new(
                        readable,
                        writable,
                        inode,
                    ))
                })
        }
    } else {
        parent_dir.find(name)
            .map(|inode| {
                if flags.contains(OpenFlags::TRUNC) {
                    inode.clear();
                }
                Arc::new(OSInode::new(
                    readable,
                    writable,
                    inode
                ))
            })
    }
}

// 新增加link
pub fn link_file(old_path:&str, new_path:&str) -> Result<(), &'static str>{
    let old_dir;
    let old_name;
    let new_dir;
    let new_name;
    let res_old = split_path(old_path);
    let res_new = split_path(new_path);
    match res_old {
        Some((inode,file_name)) => {
            old_dir = inode;
            old_name = file_name;
        }
        None => {
            return Err("Old path is not correct");
        }
    }

    match res_new {
        Some((inode,file_name)) => {
            new_dir = inode;
            new_name = file_name;
        }
        None => {
            return Err("New path is not correct");
        }
    }

    match old_dir.find_inode_number(old_name) {
        Ok(res) => {
            match res {
                Some(inode_number) => {
                    new_dir.link(new_name,inode_number);
                    Ok(())
                },
                None => Err("Can not find file to link at")
            }
        },
        Err(info) => Err(info)
    }
}

impl File for OSInode {
    fn readable(&self) -> bool { self.readable }
    fn writable(&self) -> bool { self.writable }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}
