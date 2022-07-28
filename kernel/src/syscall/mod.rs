//! 系统调用实现
//!
//! 目前的系统调用规范参照比赛所提供的类似 Linux 系统调用实现。
//! 
//! 有一些注释的系统调用名，那些是 rCore 的约定实现
//! 
//! 这两种调用间比较容易混淆的区别是，比赛测例是用 C 写的，大部分数组都是 4 Byte，
//! 而 rCore 使用 rust，usize/isize 一般是 8 Byte。
//! 这导致一些传入地址(非字符串,字符串大家都是统一的 1Byte 类型)的大小有问题，
//! 如 sys_pipe() 在测例环境下需要将输入作为 *mut u32 而不是 *mut usize

#![deny(missing_docs)]

const SYSCALL_GETCWD: usize = 17;
//const SYSCALL_DUP: usize = 24;
const SYSCALL_DUP: usize = 23;
const SYSCALL_DUP3: usize = 24;
const SYSCALL_FCNTL64: usize = 25;
const SYSCALL_IOCTL: usize = 29;
const SYSCALL_MKDIR: usize = 34;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_UMOUNT: usize = 39;
const SYSCALL_MOUNT: usize = 40;
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_GETDENTS64: usize = 61;
const SYSCALL_LSEEK: usize = 62;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_READV: usize = 65;
const SYSCALL_WRITEV: usize = 66;
const SYSCALL_SENDFILE64: usize = 71;
const SYSCALL_READLINKAT: usize = 78;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_UTIMENSAT: usize = 88;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_EXIT_GROUP: usize = 94;
const SYSCALL_SET_TID_ADDRESS: usize = 96;
const SYSCALL_FUTEX: usize = 98;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_CLOCK_GET_TIME: usize = 113;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_KILL: usize = 129;
const SYSCALL_TKILL: usize = 130;
const SYSCALL_SIGACTION: usize = 134;
const SYSCALL_SIGPROCMASK: usize = 135;
const SYSCALL_SIGTIMEDWAIT: usize = 137;
const SYSCALL_SIGRETURN: usize = 139;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_UNAME: usize = 160;
//const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GET_TIME_OF_DAY: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_GETUID: usize = 174;
const SYSCALL_GETEUID: usize = 175;
const SYSCALL_GETGID: usize = 176;
const SYSCALL_GETEGID: usize = 177;
const SYSCALL_GETTID: usize = 178;
const SYSCALL_BRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
//const SYSCALL_FORK: usize = 220;
const SYSCALL_CLONE: usize = 220;
//const SYSCALL_EXEC: usize = 221;
const SYSCALL_EXECVE: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MPROTECT: usize = 226;
//const SYSCALL_WAITPID: usize = 260;
const SYSCALL_WAIT4: usize = 260;
const SYSCALL_PRLIMIT64: usize = 261;

mod fs;
mod process;
mod flags;
mod times;

use fs::*;
use process::*;
use flags::*;
use times::*;

use lock::Mutex;
use lazy_static::*;
use crate::constants::IS_TEST_ENV;
use crate::signal::SigAction;

lazy_static! {
    static ref WRITEV_COUNT:Mutex<usize> = Mutex::new(0);
}

/// 处理系统调用
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    info!("[[kernel syscall {}]]", syscall_id);
    if IS_TEST_ENV {
        // libc-test 在某些 syscall 没有正确实现的时候，会不断循环调用 writev
        // 为了避免内核死循环，这种情况下要手动结束进程
        if syscall_id == SYSCALL_WRITEV {
            *WRITEV_COUNT.lock() += 1;
            if *WRITEV_COUNT.lock() >= 10 {
                sys_exit(-100);
            }
        } else {
            *WRITEV_COUNT.lock() = 0;
        }

        if syscall_id == SYSCALL_MMAP {
            info!("prot {:x} flags {:x}", args[2], args[3]);
        }
    }
    let a0 = match syscall_id {
        SYSCALL_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_DUP3 => sys_dup3(args[0], args[1]),
        SYSCALL_UNLINKAT => sys_unlinkat(args[0] as i32, args[1] as *const u8, args[2] as u32),
        SYSCALL_LINKAT => sys_linkat(args[0] as i32, args[1] as *const u8, args[2] as i32, args[3] as *const u8, args[4] as u32),
        SYSCALL_UMOUNT => sys_umount(args[0] as *const u8, args[1] as u32),
        SYSCALL_MOUNT => sys_mount(args[0] as *const u8, args[1] as *const u8, args[2] as *const u8, args[3] as u32, args[4] as *const u8),
        SYSCALL_MKDIR => sys_mkdir(args[0] as i32, args[1] as *const u8, args[2] as u32),
        SYSCALL_CHDIR => sys_chdir(args[0] as *const u8),
        SYSCALL_OPEN => sys_open(args[0] as i32, args[1] as *const u8, args[2] as u32, args[3] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut u32),
        SYSCALL_GETDENTS64 => sys_getdents64(args[0], args[1] as *mut Dirent64, args[2]),
        SYSCALL_LSEEK =>sys_lseek(args[0], args[1] as isize, args[2] as isize),
        SYSCALL_READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_READV => sys_readv(args[0], args[1] as *mut IoVec, args[2]),
        SYSCALL_WRITEV => sys_writev(args[0], args[1] as *const IoVec, args[2]),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut crate::file::Kstat),
        SYSCALL_UTIMENSAT => sys_utimensat(args[0] as i32, args[1] as *const u8, args[2] as *const TimeSpec, UtimensatFlags::from_bits(args[3] as u32).unwrap()),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_EXIT_GROUP => sys_exit(args[0] as i32),
        SYSCALL_NANOSLEEP => sys_nanosleep(args[0] as *const TimeSpec, args[1] as *mut TimeSpec),
        SYSCALL_CLOCK_GET_TIME => sys_get_time_of_day(args[1] as *mut TimeSpec),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0] as isize, args[1] as isize),
        SYSCALL_TKILL => sys_tkill(args[0] as isize, args[1] as isize),
        SYSCALL_SIGACTION => sys_sigaction(args[0], args[1] as *const SigAction, args[2] as *mut SigAction),
        SYSCALL_SIGPROCMASK => sys_sigprocmask(args[0] as i32, args[1] as *const usize, args[2] as *mut usize, args[3]),
        SYSCALL_SIGRETURN => sys_sigreturn(),
        SYSCALL_TIMES => sys_times(args[0] as *mut TMS),
        SYSCALL_UNAME => sys_uname(args[0] as *mut UtsName),
        SYSCALL_GET_TIME_OF_DAY => sys_get_time_of_day(args[0] as *mut TimeSpec),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_GETPPID => sys_getppid(),
        SYSCALL_GETUID => sys_getuid(),
        SYSCALL_GETEUID => sys_geteuid(),
        SYSCALL_GETGID => sys_getgid(),
        SYSCALL_GETEGID => sys_getegid(),
        SYSCALL_GETTID => sys_gettid(),
        SYSCALL_BRK => sys_brk(args[0]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_CLONE => sys_clone(args[0], args[1], args[2] as u32, args[3] as u32, args[4] as u32),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], MMAPPROT::from_bits(args[2] as u32).unwrap(), MMAPFlags::from_bits(args[3] as u32).unwrap(), args[4] as i32, args[5]),
        SYSCALL_EXECVE => sys_execve(args[0] as *const u8, args[1] as *const usize, args[2] as *const usize),
        SYSCALL_WAIT4 => sys_wait4(args[0] as isize, args[1] as *mut i32, WaitFlags::from_bits(args[2] as u32).unwrap()),
        //_ => panic!("Unsupported syscall_id: {}", syscall_id),
        SYSCALL_SET_TID_ADDRESS => sys_gettid(),
        SYSCALL_IOCTL => 0,
        SYSCALL_FCNTL64 => 0,
        SYSCALL_MPROTECT => 0,
        SYSCALL_FUTEX => sys_exit(-100),
        SYSCALL_SIGTIMEDWAIT => 0,
        SYSCALL_PRLIMIT64 => 0,
        _ => {
            println!("Unsupported syscall_id: {}", syscall_id);
            -1
        },
    };
    info!("[[kernel -> return {}  =0x{:x}]]", a0, a0);
    a0
}
