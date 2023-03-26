use easy_fs::{DirEntry, EasyFileSystem, FileType, Inode, OSStat};
use crate::drivers::BLOCK_DEVICE;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use lazy_static::*;
use bitflags::*;
use alloc::vec::Vec;
use super::File;
use crate::mm::UserBuffer;
use crate::task::current_task;

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

/*// 对路径检查，得到不含"./"的路径，如果含有“../”返回false
#[inline]
fn check_path(path: &mut Vec<&str>) -> bool {
    if path[0] == "." {
        path.remove(0);
    }
    if path[0] == ".." {
        return false;
    }
    true
}*/

#[inline]
fn parent_dir_ino() -> u32 {
    let tcb = current_task().unwrap();
    let inner = tcb.inner_exclusive_access();
    inner.current_dir_ino
}

fn from_inode_id(inode_id:u32) -> Inode {
    let efs = ROOT_INODE.get_fs();
    let block_device = ROOT_INODE.get_device();
    let (block_id, block_offset) = efs.lock().get_disk_inode_pos(inode_id);
    Inode::new(
        block_id,
        block_offset,
        efs,
        block_device,
    )
}

fn current_dir_inode() -> Inode {
    let ino = parent_dir_ino();
    from_inode_id(ino)
}

// 解析包含了“./”和“../”的路径字符串，
// 返回它相对路径开始的第一个目录的Inode，
// 和以此目录为起点的相对路径
fn derive_path(path:&str) -> (Arc<Inode>,Vec<&str>) {
    if path.starts_with('/') {
        // 绝对路径解析
        let path:Vec<_> = path.split('/').collect();
        (ROOT_INODE.clone(), path)
    }
    else {
        // 相对路径
        let mut path:Vec<_> = path.split('/').collect();
        let mut parent_dir_ino = current_dir_inode();
        let mut count = 0;
        for (i,dir) in path.iter().enumerate(){
            if dir.eq(&"..") {
                parent_dir_ino = parent_dir_ino.parent_inode();
            }
            else if dir.eq(&".") {
                continue
            }
            else {
                count = i;
                break
            }
        }
        (Arc::new(parent_dir_ino), path.drain(count..).collect())
    }
}

// dir_path为上层目录数组
fn find_inode(inode: Arc<Inode>, dir_path: &[&str]) -> Option<Arc<Inode>> {
    assert!(dir_path.len() >= 1);
    let next_dir = inode.find(dir_path[0]);
    match next_dir {
        Some(dir) => {
            if dir_path.len() == 1 {
                // 递归出口
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
    let (parent_dir_inode,path) = derive_path(path);
    let parent_dir_inode = find_inode(
        parent_dir_inode,
        &path.as_slice()[..(path.len() - 2)]
    );
    match parent_dir_inode {
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

pub fn unlink(path: &str) -> Result<(), &'static str> {
    let res = split_path(path);
    match res {
        Some((inode,file_name)) => {
            inode.remove(file_name)
        }
        None => Err("Can not find path")
    }
}

pub fn create_dir(path: &str) -> Option<Arc<OSInode>> {
    let res = split_path(path);
    match res {
        Some((inode,file_name)) => {
            let res = inode.create_dir(file_name);
            match res {
                Some(inner) => Some(Arc::new(OSInode::new(
                    true,
                    false,
                    inner
                ))),
                None => None
            }
        }
        None => None
    }
}

/*// 对目录只读
pub fn open_dir(path:&str) -> Option<Arc<OSInode>> {
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
    parent_dir.find(name)
        .map(|inode| {
            Arc::new(OSInode::new(
                true,
                false,
                inode
            ))
        })
}*/

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
    // whence第1位为1表示将OSInode的offset设置为当前的offset
    // whence第2位为1表示将OSInode的offset将传入的的offset
    fn seek(&self, offset: usize, whence: usize) -> isize {
        let mut inner = self.inner.exclusive_access();
        if whence & 1 > 0 {
            inner.offset = offset;
            inner.offset as isize
        }
        else if whence & (1 << 1) > 0 {
            inner.offset += offset;
            inner.offset as isize
        }
        else {
            -1
        }
    }

    fn stat(&self) -> Option<OSStat> {
        let inner = self.inner.exclusive_access();
        Some(inner.inode.fstat())
    }
}



// 为了实现目录的读取的读取，增加一个接口
pub trait Directory {
    fn is_dir(&self) -> bool {
        false
    }
    fn get_dents(&self, _count:usize) -> Option<Vec<OSDirent>> {
        None
    }
}

//type FileType = DiskInodeType;

// 拓展目录项，比索引块中的目录项DirEntry有更多的的信息
pub struct OSDirent {
    d_ino: u32,
    d_name: [u8; 28],
    d_offset: usize,
    d_type: FileType,
}

fn dent2os(dirent:DirEntry, index:usize) -> OSDirent {
    let d_ino = dirent.inode_number;
    let inode = from_inode_id(d_ino);
    let d_type = inode.read_disk_inode(|disk_inode|{
        disk_inode.type_.clone()
    });
    OSDirent {
        d_ino,
        d_name: dirent.name,
        d_offset: index,
        d_type,
    }
}

impl Directory for OSInode {
    fn is_dir(&self) -> bool {
        let inner = self.inner.exclusive_access();
        inner.inode.is_dir()
    }

    fn get_dents(&self, count:usize) -> Option<Vec<OSDirent>> {
        let inner = self.inner.exclusive_access();
        if inner.inode.is_file() {
            None
        }
        else {
            let dents = inner.inode.get_dents(count);
            let mut v:Vec<OSDirent> = Vec::with_capacity(dents.len());
            let mut i = 0;
            for dent in dents{
                v.push(dent2os(dent,i));
                i +=1;
            }
            Some(v)
        }
    }
}

pub trait Fd: File + Directory {}

impl Fd for OSInode {}
