use core::fmt::Arguments;

#[inline]
pub fn print(args: Arguments) {
    crate::arch::stdout::stdout_puts(args);
}

#[inline]
pub fn info(args: Arguments) {
    if crate::constants::BASE_INFO {
        crate::arch::stdout::stdout_puts(args);
    }
}

#[inline]
pub fn error_print(args: Arguments) {
    crate::arch::stdout::stderr_puts(args);
}

/// 仅在开启信息输出时，打印格式字串
#[macro_export]
macro_rules! info {
    () => ($crate::console::info(core::format_args!("\n")););
    ($($arg:tt)*) => {
        $crate::console::info(core::format_args!($($arg)*));
        $crate::info!();
    }
}

/// 打印格式字串，无换行
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::print(core::format_args!($($arg)*));
    }
}

/// 打印格式字串，使用与 print 不同的 Mutex 锁
#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {
        $crate::console::error_print(core::format_args!($($arg)*));
    }
}

/// 打印格式字串，有换行
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {
        $crate::console::print(core::format_args!($($arg)*));
        $crate::println!();
    }
}

/// 打印格式字串，使用与 println 不同的 Mutex 锁
#[macro_export]
macro_rules! eprintln {
        () => ($crate::eprint!("\n"));
    ($($arg:tt)*) => {
        $crate::console::error_print(core::format_args!($($arg)*));
        $crate::eprintln!();
    }
}
