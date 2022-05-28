use core::fmt::Arguments;

#[allow(dead_code)]
pub fn print(args: Arguments) {
    crate::arch::stdout::stdout_puts(args);
}

#[allow(dead_code)]
pub fn info(args: Arguments) {
    if crate::constants::BASE_INFO {
        crate::arch::stdout::stdout_puts(args);
    }
}

#[allow(dead_code)]
pub fn error_print(args: Arguments) {
    crate::arch::stdout::stderr_puts(args);
}

/// 仅在开启信息输出时，打印格式字串
#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::info(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

/// 打印格式字串，无换行
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

/// 打印格式字串，使用与 print 不同的 Mutex 锁
#[macro_export]
macro_rules! error_print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::error_print(format_args!($fmt $(, $($arg)+)?));
    }
}

/// 打印格式字串，有换行
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

/// 打印格式字串，使用与 println 不同的 Mutex 锁
#[macro_export]
macro_rules! error_println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::error_print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
