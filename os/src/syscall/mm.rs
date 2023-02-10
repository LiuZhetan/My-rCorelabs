use crate::mm::{MapPermission, VirtAddr};
use crate::task::{cross_current_memset, current_memset_push, in_current_memset};

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    //检查prot的合法性
    if (prot & !0x7 != 0) || (prot & 0x7 == 0) {
        return -1
    }
    let start_va: VirtAddr = start.into();
    let end_va: VirtAddr = (start + len).into();

    if start_va.page_offset() > 0 {
        return -1
    }

    if cross_current_memset(start_va.into(), end_va.floor().into()) {
        return -1;
    }

    // 分配frame
    let mut permission = MapPermission::R | MapPermission::U;
    if prot & 0x2 != 0 {
        permission |= MapPermission::W;
    }
    if prot & 0x4 != 0 {
        permission |= MapPermission::X;
    }
    current_memset_push(start_va,end_va,permission);
    0
}

/// munmap要复杂一点，涉及到段的分割
pub fn sys_munmap(start: usize, len: usize) -> isize {
    0
}