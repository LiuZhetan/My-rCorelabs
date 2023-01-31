#![allow(unused)]

///risc v SBI用于进入M级别的特权
///risc v SBI reference: https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/riscv-sbi.adoc

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_SHUTDOWN: usize = 8;

/*enum  SBIResult {
    Error(i32),
    Value(i32),
}*/

#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    /*
    sbi call实际上会返回error和value，分别位于a0和a1，
    但是本函数使用Legacy Extensions只需要将FID置0,并只返回a0
    a6放FID,a7放EID
     */
    let mut ret;
    unsafe {
        core::arch::asm!(
        "li x16, 0",    // li: load immediate
        "ecall",
        inlateout("a0") arg0 => ret,
        in("a1") arg1,
        in("a2") arg2,
        in("a7") which,
        );
    }
    ret
}

pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    panic!("It should shutdown!");
}