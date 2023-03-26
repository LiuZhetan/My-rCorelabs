use alloc::string::String;
use crate::mm::{
    UserBuffer,
    translated_byte_buffer,
    translated_refmut,
    translated_str,
};
use crate::task::{current_user_token, current_task};
use crate::fs::{make_pipe, OpenFlags, open_file, create_dir, OSDirent, link_file, unlink};
use alloc::sync::Arc;
use alloc::vec::Vec;
use easy_fs::{FileType};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(
        path.as_str(),
        OpenFlags::from_bits(flags).unwrap()
    ) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
}

pub fn sys_linkat(old_path:*const u8, new_path:*const u8) -> isize{
    let token = current_user_token();
    let old_path = translated_str(token, old_path);
    let new_path = translated_str(token, new_path);
    match link_file(old_path.as_str(), new_path.as_str()) {
        Ok(_) => 0,
        Err(info) => {
            println!("[Kernel] Error: {}",info);
            -1
        }
    }
}

pub fn sys_unlinkat(path:*const u8) -> isize{
    let token = current_user_token();
    let path = translated_str(token, path);
    match unlink(path.as_str()) {
        Ok(_) => 0,
        Err(info) => {
            println!("[Kernel] Error: {}",info);
            -1
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}

// StatMode 定义：
bitflags! {
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

pub fn sys_stat(fd: usize, st: *mut Stat) -> isize{
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let st = translated_refmut(token, st);
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        match file.stat() {
            Some(os_st) => unsafe {
                st.dev = 0;
                st.ino = os_st.ino as u64;
                match os_st.f_type {
                    FileType::File => st.mode = StatMode::FILE,
                    FileType::Directory => st.mode = StatMode::DIR,
                }
                st.nlink = os_st.nlink;
                0
            },
            None => -1
        }
    } else {
        -1
    }
}

// 新增对于目录的系统调用
pub fn sys_mkdir(path:*const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(_) = create_dir(
        path.as_str()
    ) {
        0
    } else {
        -1
    }
}

// buf指向存放OSDirent的缓冲区,
// count是预读目录项的数目，
// isize是实际读取的目录项树木,
// 返回-1表示读取失败
pub fn sys_getdents(fd:usize, buf: *const OSDirent, count: usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() || !file.is_dir() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        let res = file.get_dents(count);
        match res {
            Some(dents) => {
                // 把dents复制到buf指向的缓冲区
                let buf = translated_refmut(token,buf as *mut OSDirent) as *mut OSDirent;
                let count = dents.len();
                let mut buf = unsafe{Vec::from_raw_parts(buf,count,count)};
                let mut i = 0;
                for dent in dents {
                    buf[i] = dent;
                    i += 1;
                }
                count as isize
            }
            None => {
                -1
            }
        }
    } else {
        -1
    }
}

pub fn sys_fseek(fd:usize, offset: usize, whence: usize) -> isize{
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        return if !file.readable() || file.is_dir() {
            -1
        } else {
            file.seek(offset, whence)
        }
    }
    -1
}

/*
pub fn sys_open_dir(path: *const u8) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_dir(
        path.as_str()
    ) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}*/

// 获取当前进程的工作目录
pub fn sys_getcwd() -> String {
    let tcb = current_task().unwrap();
    let inner = tcb.inner_exclusive_access();
    inner.current_dir.clone()
}

// 路径转绝对路径
fn path_to_absolute(path:&str) -> String {
    if path.starts_with("/") {
        String::from(path)
    }
    else {
        let cwd = sys_getcwd();
        let mut cwd_splits: Vec<_> = cwd.split('/').collect();
        let mut path_splits: Vec<_> = path.split('/').collect();
        let mut count = 0;
        for (i, split) in path_splits.iter().enumerate() {
            if (*split).eq("..") {
                cwd_splits.pop();
            }
            else if (*split).eq(".") {
                continue
            }
            else {
                count = i;
                break
            }
        }
        let mut path_splits: Vec<_> = path_splits.drain(count..).collect();
        cwd_splits.append(&mut path_splits);
        let mut res = String::from("/");
        for str in cwd_splits {
            res += str;
        }
        res
    }
}

// 改变当前进程的工作路径
pub fn sys_chdir(path:&str) {
    //将path变为绝对路径
    let path = path_to_absolute(path);
    let tcb = current_task().unwrap();
    let mut inner = tcb.inner_exclusive_access();
    inner.current_dir = path;
}