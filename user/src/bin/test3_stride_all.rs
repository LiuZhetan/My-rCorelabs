#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use alloc::string::{String, ToString};
use user_lib::{getpid, wait, spawn};

#[no_mangle]
pub fn main() -> i32 {
    println!("start running all stride tests ...");
    let path = String::from("test3_stride");
    for i in 0..=5 {
        let path_i = path.clone() + &*i.to_string();
        println!("run {}",path_i);
        let pid = spawn(path_i.as_str());
    }

    let mut exit_code: i32 = 0;
    for _ in 0..=5 {
        if wait(&mut exit_code) <= 0 {
            panic!("wait stopped early");
        }
    }
    if wait(&mut exit_code) > 0 {
        panic!("wait got too many");
    }
    println!("finished stride test");
    0
}


/*
one test result:
priority = 7, exitcode = 3808000, p/c = 544000
priority = 6, exitcode = 3536000, p/c = 589333
priority = 5, exitcode = 2894800, p/c = 578960
priority = 8, exitcode = 5138800, p/c = 642350
priority = 9, exitcode = 5877600, p/c = 653066
priority = 10, exitcode = 6550400, p/c = 655040

655040 / 544000 = 1.2 < 1.5
pass test
*/