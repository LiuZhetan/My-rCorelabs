use crate::mm::MapPermission;
use crate::task::{current_mmap, current_unmap};

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    //检查prot的合法性
    if (prot & !0x7 != 0) || (prot & 0x7 == 0) {
        return -1
    }
    let start_va = start;
    let end_va= start + len;

    if current_mmap(start_va, end_va,prot) {
        0
    }
    else {
        -1
    }
}

/// munmap要复杂一点，涉及到段的分割
pub fn sys_munmap(start: usize, len: usize) -> isize {
    let start_va = start;
    let end_va= start + len;
    if current_unmap(start_va,end_va) {
        0
    }
    else {
        -1
    }
}