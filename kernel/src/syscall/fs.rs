//! File and filesystem-related syscalls

const FD_STDOUT: usize = 1;

use super::phys_to_virt;
/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            println!("sys_write at {:x}", (buf as usize) % 0x1_0000_0000);
            return len as isize;
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
