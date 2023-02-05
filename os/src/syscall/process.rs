//! Process management syscalls
use core::arch::asm;
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::{get_time_ms};
//use crate::trap::enable_timer_interrupt;

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    println!("[kernel] Application yield");
    suspend_current_and_run_next();
    0
}

/// get time in milliseconds
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

unsafe fn enable_interrupt() {
    let mut sstatus:usize;
    asm!("csrr {}, sstatus", out(reg) sstatus);
    sstatus = sstatus | (1 << 1);
    asm!("csrw sstatus, {}", in(reg) sstatus);
}

unsafe fn disable_interrupt() {
    let mut sstatus:usize;
    asm!("csrr {}, sstatus", out(reg) sstatus);
    sstatus = sstatus ^ (1 << 1);
    sstatus = sstatus ^ (1 << 5);
    asm!("csrw sstatus, {}", in(reg) sstatus);
}

pub fn sys_loop() -> isize {
    let mut sstatus:usize;
    unsafe {
        asm!("csrr {}, sstatus", out(reg) sstatus)
    };
    println!("sstatus: {:#x}", sstatus);
    sstatus = sstatus | (1 << 1);
    println!("sstatus: {:#x}", sstatus);
    unsafe {
        asm!("csrw sstatus, {}", in(reg) sstatus)
    };

    for _i in 1..1000 {
        //println!("running in kernel");
        let mut sp:usize;
        unsafe {
            asm!("mv {}, sp", out(reg) sp);
            asm!("csrr {}, sstatus", out(reg) sstatus);
        };
        println!("current kernel sp {:#x}",sp);
        println!("current sstatus {:#x}",sstatus);
    }

    sstatus = sstatus ^ (1 << 1);
    sstatus = sstatus ^ (1 << 5);
    println!("Write sstatus: {:#x}", sstatus);
    unsafe {
        asm!("csrw sstatus, {}", in(reg) sstatus)
    };
    unsafe {
        asm!("csrr {}, sstatus", out(reg) sstatus)
    };
    println!("Read sstatus: {:#x}", sstatus);
    loop {

    }
}

/*pub fn test_fun() -> (){
    loop {
        println!("[kernel] this is a test");
    };
}*/

pub unsafe fn sys_task<F>(fun:F) -> isize
    where F: FnOnce() -> () {
    println!("[kernel] run func");
    enable_interrupt();
    fun();
    disable_interrupt();
    println!("[kernel] finish run func");
    0
}
