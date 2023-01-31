//!不同于U模式，S模式下需要为U模式的系统调用提供地层服务

mod fs;
mod process;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

pub fn syscall(syscall_number:usize, args:[usize;3]) -> isize {
    match syscall_number {
        SYSCALL_WRITE => fs::sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => process::sys_exit(args[0] as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_number),
    }
}