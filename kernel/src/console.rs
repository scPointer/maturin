use core::fmt::Arguments;
use log::*;

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
macro_rules! base_info {
    () => ($crate::console::info(core::format_args!("\n")););
    ($($arg:tt)*) => {
        $crate::console::info(core::format_args!($($arg)*));
        $crate::base_info!();
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

static LOGGER: SimpleLogger = SimpleLogger;

pub fn init_logger(on: bool) -> Result<(), SetLoggerError> {
    let level = if on {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };
    set_logger(&LOGGER).map(|()| set_max_level(level))
}
struct SimpleLogger;
impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "\x1b[{}m {} - {} \x1b[0m",
                level_to_color_code(record.level()),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
fn level_to_color_code(level: Level) -> u8 {
    match level {
        Level::Error => 31, // Red
        Level::Warn => 33,  // Yellow
        Level::Info => 32,  // Green
        Level::Debug => 36, // SkyBlue
        Level::Trace => 90, // BrightBlack
    }
}
