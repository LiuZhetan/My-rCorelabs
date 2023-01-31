#![feature(panic_info_message)]
#![no_main]
#![no_std]

use core::arch::global_asm;


#[macro_use]
mod console;
mod lang_items;
mod syscall;
mod sbi;
mod logging;
mod sync;
mod batch;
mod board;
mod trap;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

fn clear_bss(){
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize .. ebss as usize).for_each(
        |a| {unsafe {(a as *mut u8).write_volatile(0)}}
    );
}

// 一定要加no_mangle属性告诉编译器不要修改函数名称
#[no_mangle]
pub fn rust_main() -> !{
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    batch::init();
    batch::run_next_app();
}

//qemu运行命令：
/*
qemu-system-riscv64 \
            -machine virt \
            -nographic \
            -bios ../bootloader/rustsbi-qemu.bin \
            -device loader,file=target/riscv64gc-unknown-none-elf/debug/os,addr=0x80200000
*/