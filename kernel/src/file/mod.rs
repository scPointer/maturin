//! 文件类抽象，包含文件系统、stdin/stdout、管道等

mod backend;
mod device;
mod fd_manager;
mod fs_stat;
mod pipe;
mod poll_events;
mod stdio;
mod vfs;
pub mod socket;

pub use fatfs::SeekFrom;

pub use device::{
    check_dir_exists,
    check_file_exists,
    fs_init,
    get_dir_entry_iter,
    list_files_at_root,
    //load_testcases,
    load_next_testcase,
    mkdir,
    mount_fat_fs,
    open_file,
    origin_fs_stat,
    show_testcase_result,
    try_add_link,
    try_remove_link,
    read_link,
    umount_fat_fs,
    rename_or_move,
    add_sys_info,
};

pub use backend::{BackEndFile, SyncPolicy};
pub use device::{FatFile, FileDisc};
pub use fd_manager::FdManager;
pub use fs_stat::FsStat;
pub use pipe::{Pipe, RingBuffer};
pub use poll_events::PollEvents;
pub use socket::Socket;
pub use vfs::{
    BufferFile,
    get_virt_file_if_possible,
    get_virt_dir_if_possible,
    check_virt_dir_exists,
    check_virt_file_exists,
    try_make_virt_dir,
    try_remove_virt_file,
};
