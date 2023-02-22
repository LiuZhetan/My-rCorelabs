#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{getpid, wait, spawn};

#[no_mangle]
pub fn main() -> i32 {
    println!("pid {}: parent start spawning ...", getpid());
    let pid = spawn("hello_world");
    if pid > 0 {
        // sucess
        let mut exit_code: i32 = 0;
        println!("pid {}: ready waiting child ...", getpid());
        assert_eq!(pid, wait(&mut exit_code));
        assert_eq!(exit_code, 0);
        println!(
            "pid {}: got child info:: pid {}, exit code: {}",
            getpid(),
            pid,
            exit_code
        );
        100
    } else {
        println!("can not create child process");
        -1
    }
}