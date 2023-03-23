use alloc::string::String;
use crate::mm::{
    UserBuffer,
    translated_byte_buffer,
    translated_refmut,
    translated_str,
};
use crate::task::{current_user_token, current_task};
use crate::fs::{make_pipe, OpenFlags, open_file, create_dir};
use alloc::sync::Arc;
use alloc::vec::Vec;
use easy_fs::{DIRENT_SZ,DirEntry};

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

struct OSDirent {
    name: [u8; 28],
    inode_number: u32,
}

pub unsafe fn sys_getdents(fd:usize, buf: *const u8, count: usize) -> isize {
    // 读取原有的Dirent
    let dirent_buf:Vec<DirEntry> = Vec::with_capacity(count);
    // 不能用已内核中的地址
    let read = sys_read(fd,dirent_buf.as_ptr() as *const u8,DIRENT_SZ * count);
    if read <= 0 {
        return -1;
    }
    let v: Vec<OSDirent> = Vec::from_raw_parts(buf as *mut OSDirent,count,count);
    for  in  {

    }
    0
}

pub fn sys_fseek() {

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