use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_SET_PRIORITY: usize = 140;
const SYSCALL_TASK_INFO: usize = 410;


fn sys_call(syscall_number:usize, args:[usize;3]) -> isize {
    let mut ret;
    unsafe {
        asm!(
        "ecall",
        in("a7") syscall_number,
        inlateout("a0") args[0] => ret,
        in("a1") args[1],
        in("a2") args[2],
        );
    }
    return ret;
}

pub fn sys_exit(xstate:i32) -> isize {
    sys_call(SYSCALL_EXIT, [xstate as usize,0,0])
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    //使用as_ptr获得原始指针
    sys_call(SYSCALL_WRITE,[fd,buffer.as_ptr() as usize,buffer.len()])
}