#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::loop_test;

#[no_mangle]
fn main() -> i32 {
    loop_test();
    0
}