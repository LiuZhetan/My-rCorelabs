use core::fmt;
use core::fmt::{Write, Arguments};
use crate::sbi::console_putchar;

struct Stdout;

//实现Write trait
impl Write for Stdout {
    //在qemu-system-riscv64上调用此系统调用似乎没有反映
    /*fn write_str(&mut self, s:&str) -> Result{
        sys_write(1,s.as_bytes());
        Ok(())
    }*/
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: Arguments) {
    Stdout.write_fmt(args).unwrap();
}

macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}