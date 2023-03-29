//! redis 对应的系统调用
//! 
//! 目前默认是先起 busybox 再起 redis，此时 redis 的 PID=3

//#![deny(missing_docs)]

use core::panic;

//use base_file::Kstat;
use epoll::EpollEvent;
use super::*;
use timer::TimeSpec;

//use crate::file::FsStat;
//use crate::signal::SigAction;

/// 处理系统调用
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    let syscall_id = if let Ok(id) = SyscallNo::try_from(syscall_id) {
        id
    } else {
        error!(
            "Unsupported syscall id = {:#?}({})",
            syscall_id, syscall_id as usize
        );
        return 0;
    };
    debug!("Syscall {:?}, {:x?}", syscall_id, args);
    if syscall_id != SyscallNo::CLOCK_GET_TIME
        && syscall_id != SyscallNo::EPOLL_WAIT
        && syscall_id != SyscallNo::FUTEX
        && syscall_id != SyscallNo::OPEN
    {
        print!("Syscall {:?}, {:x?}", syscall_id, args);
    }
    if syscall_id == SyscallNo::EPOLL_WAIT {
        panic!("redis init Ok, exit")
    }
    let result = match syscall_id {

        // memory
        SyscallNo::BRK => sys_brk(args[0]),
        SyscallNo::MUNMAP => sys_munmap(args[0], args[1]),
        SyscallNo::MMAP => {
            let prot = if args[2] == 0 {
                MMAPPROT::PROT_READ | MMAPPROT::PROT_WRITE
            } else {
                MMAPPROT::from_bits(args[2] as u32).unwrap()
            };
            let flags = MMAPFlags::from_bits(args[3] as u32).unwrap();
            if !flags.contains(MMAPFlags::MAP_PRIVATE | MMAPFlags::MAP_ANONYMOUS) {
                panic!("need advanced mmap flags = {:#?}", flags);
            }
            sys_mmap(
            args[0],
            args[1],
            prot,
            flags,
            args[4] as i32,
            args[5],
        )},
        SyscallNo::EXECVE => sys_execve( // 这里是 shell 去启动 redis 的时候调用的，不需要管
            args[0] as *const u8,
            args[1] as *const usize,
            args[2] as *const usize,
        ),

        // thread and lock
        SyscallNo::SET_TID_ADDRESS => sys_set_tid_address(args[0]),
        SyscallNo::CLONE => sys_clone(args[0], args[1], args[2], args[3], args[4]),
        SyscallNo::FUTEX => sys_futex(
            args[0],
            args[1] as i32,
            args[2] as u32,
            args[3],
            args[4],
            args[5] as u32,
        ),
        // process
        SyscallNo::EXIT | SyscallNo::EXIT_GROUP => sys_exit(args[0] as i32),
        SyscallNo::CLOCK_GET_TIME => timer::sys_clock_gettime(args[0], args[1] as *mut TimeSpec),
        SyscallNo::YIELD => sys_yield(),
        SyscallNo::GETPID => sys_getpid(),
        
        // IO and FS
        SyscallNo::READ => sys_read(args[0], args[1] as *mut u8, args[2]),
        SyscallNo::WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SyscallNo::WRITEV => sys_writev(args[0], args[1] as *const IoVec, args[2]),
        SyscallNo::OPEN => sys_open(
            args[0] as i32,
            args[1] as *const u8,
            args[2] as u32,
            args[3] as i32,
        ),
        SyscallNo::CLOSE => sys_close(args[0]),
        SyscallNo::PIPE => sys_pipe(args[0] as *mut u32),
        SyscallNo::GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
        SyscallNo::FCNTL64 => sys_fcntl64(args[0], args[1], args[2]),

        // epoll
        SyscallNo::EPOLL_CREATE => epoll::sys_epoll_create(args[0]),
        SyscallNo::EPOLL_CTL => epoll::sys_epoll_ctl(
            args[0] as i32,
            args[1] as i32,
            args[2] as i32,
            args[3] as *const EpollEvent,
        ),
        SyscallNo::EPOLL_WAIT => epoll::sys_epoll_wait(
            args[0] as i32,
            args[1] as *mut EpollEvent,
            args[2] as i32,
            args[3] as i32,
        ),
        
        //socket
        SyscallNo::SOCKET => sys_socket(args[0], args[1], args[2]),
        SyscallNo::SENDTO => sys_sendto(
            args[0],
            args[1] as *const u8,
            args[2],
            args[3] as i32,
            args[4] as *const u8,
            args[5],
        ),
        SyscallNo::RECVFROM => sys_recvfrom(
            args[0],
            args[1] as *mut u8,
            args[2],
            args[3] as i32,
            args[4] as *mut u8,
            args[5] as *mut u32,
        ),
        SyscallNo::BIND => sys_bind(args[0], args[1] as *const u8, args[2]),
        SyscallNo::LISTEN => sys_listen(args[0], args[1]),
        SyscallNo::CONNECT => sys_connect(args[0], args[1] as *const u8, args[2]),
        SyscallNo::ACCEPT => sys_accept4(
            args[0],
            args[1] as *mut u8,
            args[2] as *mut u32,
            args[3] as i32,
        ),
        SyscallNo::ACCEPT4 => sys_accept4(
            args[0],
            args[1] as *mut u8,
            args[2] as *mut u32,
            args[3] as i32,
        ),
        _ => {
            //_ => panic!("Unsupported syscall id = {:#?}()", syscall_id, syscall_id as usize);
            warn!(
                "Unsupported syscall id = {:#?}({})",
                syscall_id, syscall_id as usize
            );
            Ok(0)
        }
    };
    match result {
        Ok(a0) => {
            if syscall_id != SyscallNo::GETRUSAGE && syscall_id != SyscallNo::CLOCK_GET_TIME {
                debug!("{:?} ret -> {} = {:#x}", syscall_id, a0, a0);
                if syscall_id != SyscallNo::CLOCK_GET_TIME
                    && syscall_id != SyscallNo::EPOLL_WAIT
                    && syscall_id != SyscallNo::FUTEX
                    && syscall_id != SyscallNo::OPEN
                {
                    println!(" ...ret -> {}({:#x})", a0, a0);
                }
            }
            a0 as isize
        }
        Err(num) => {
            warn!("{:?} ret -> {:?}", syscall_id, num);
            if syscall_id != SyscallNo::CLOCK_GET_TIME
                && syscall_id != SyscallNo::EPOLL_WAIT
                && syscall_id != SyscallNo::FUTEX
                && syscall_id != SyscallNo::OPEN
            {
                println!(" ...ret -> {:?}", num);
            }
            num as isize
        }
    }
}
