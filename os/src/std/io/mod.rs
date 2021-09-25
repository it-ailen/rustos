use core::fmt::{self, Write};

use crate::sbi::console_putchar;

/// 标准输出流
const stdout: usize = 1;

pub(crate) struct Stdout;

impl Write for Stdout {
    /// 调用 sys_write 系统调用，向 stdout 写数据
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut buffer = [0u8; 4];
        // 使用 sbi 方法，不能用 syscall
        for c in s.chars() {
            for code_ptr in c.encode_utf8(&mut buffer).as_bytes().iter() {
                console_putchar(*code_ptr as usize);
            }
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

/// print!(fmt, args...)
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::std::io::print(format_args!($fmt, $(, $($arg)+)?));
    };
}

/// println!(fmt, args...)
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::std::io::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    };
}
