use crate::config::PAGE_SIZE_BITS;
use crate::mm::{VirtAddr,PageTable};
use crate::task::{suspend_current_and_run_next, exit_current_and_run_next, current_user_token};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    let _us = get_time_us();
    println!("[kernel] get_time_us: {}",_us);
    let vir_address:VirtAddr = (_ts as usize).into();
    println!("[kernel] vir_address: {:#x}",vir_address.0);
    let ppn = PageTable::from_token(current_user_token())
                            .translate(vir_address.floor())
                            .unwrap()
                            .ppn();
    println!("[kernel] ppn: {:#x}",ppn.0);
    let phys_address = (ppn.0 << PAGE_SIZE_BITS) + vir_address.page_offset();
    println!("ppn_offset:{:#x}, page_offset: {:#x}",ppn.0 << PAGE_SIZE_BITS, vir_address.page_offset());
    println!("[kernel] phys_address: {:#x}",phys_address);
    let ts = phys_address as *mut TimeVal;
    unsafe {
        *ts = TimeVal {
            sec: _us / 1_000_000,
            usec: _us % 1_000_000,
        };
    }
    0
}