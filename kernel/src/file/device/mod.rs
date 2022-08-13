//! FAT文件系统设备的抽象
//! 包括读写文件等的支持

//#![deny(missing_docs)]

mod fat_dir;
mod fat_file;
mod fd_dir;
mod link;
mod open_flags;
mod stat;
mod test;

use super::{
    get_virt_file_if_possible,
    check_virt_dir_exists,
    get_virt_dir_if_possible,
    check_virt_file_exists,
    try_remove_virt_file,
    try_make_virt_dir,
    File
};
use crate::{
    constants::ROOT_DIR,
    drivers::{new_memory_mapped_fs, MemoryMappedFsIoType},
};
use alloc::{string::String, sync::Arc};
use fatfs::{DefaultTimeProvider, Error, FileSystem, LossyOemCpConverter};
use link::parse_file_name;

type FsIO = MemoryMappedFsIoType;
type FsTP = DefaultTimeProvider;
type FsOCC = LossyOemCpConverter;

type FsDir = fatfs::Dir<'static, FsIO, FsTP, FsOCC>;
type FsDirIter<'a> = fatfs::DirIter<'a, FsIO, FsTP, FsOCC>;
type FsFile = fatfs::File<'static, FsIO, FsTP, FsOCC>;
type FATFileSystem = FileSystem<FsIO, FsTP, FsOCC>;

pub use fat_dir::FatDir;
pub use fat_file::FatFile;
pub use fd_dir::FdDir;
pub use link::FileDisc;
pub use link::{get_link_count, mount_fat_fs, try_add_link, try_remove_link, umount_fat_fs};
pub use open_flags::OpenFlags;
pub use stat::get_fs_stat as origin_fs_stat;
pub use test::{
    //load_testcases,
    load_next_testcase,
    show_testcase_result,
};

lazy_static::lazy_static! {
    //static ref MEMORY_FS: Arc<Mutex<FileSystem<FsIO, FsTP, FsOCC>>> = Arc::new(Mutex::new(new_memory_mapped_fs()));
    static ref MEMORY_FS: FATFileSystem = new_memory_mapped_fs();
}

/// 输出根目录下的所有文件
///
/// 注意，这个函数的输出是 info，这表示不打开 info 时它什么都不会输出
pub fn list_files_at_root() {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    for dir_entry in root.iter() {
        let file = dir_entry.unwrap();
        info!("file: {}", file.file_name());
        // 如果是子目录，则再继续遍历
        if file.is_dir() {
            info!("dir: {}/", file.file_name());
            for dir_entry in root.open_dir(file.file_name().as_str()).unwrap().iter() {
                let file = dir_entry.unwrap();
                // "." 开头的是当前目录、父目录以及(未来可能的)隐藏文件
                if !file.file_name().starts_with(".") {
                    info!("\tfile: {}", file.file_name());
                }
            }
        }
    }
}

/// 初始化硬盘内容。
/// 由于它需要调用 MEMORY_FS，所以不能塞进其它初始化过程里
pub fn fs_init() {
    //mkdir(ROOT_DIR, "tmp");
    mkdir(ROOT_DIR, "dev");
    mkdir(ROOT_DIR, "lib");
    
    mkdir("dev/", "shm");
    let dso = &"tls_get_new-dtv_dso.so"; // dtv 不会在根目录下找，而是会去 lib 等目录找，所以需要链接
    let libc_so = &"ld-musl-riscv64-sf.so.1";
    try_add_link(ROOT_DIR.into(), dso, "./lib/".into(), dso);
    try_add_link(
        ROOT_DIR.into(),
        "libc.so",
        "./lib/".into(),
        libc_so
    );
    mkdir(ROOT_DIR, "sbin");
    try_add_link(ROOT_DIR.into(), "lmbench_all".into(), "./sbin/".into(), "lmbench_all".into()); // busybox会去这里找
    try_add_link(ROOT_DIR.into(), "busybox".into(), "./sbin/".into(), "busybox".into());
    // 一些系统信息文件。todo: 更正确的方式应该是放到 vfs 里而不是直接塞 fat32 这边
    mkdir(ROOT_DIR, "proc"); // 进程状态信息
    let _meminfo = open_file("./proc/", "meminfo", OpenFlags::CREATE).unwrap(); // 内存占用信息
    let _mounts = open_file("./proc/", "mounts", OpenFlags::CREATE).unwrap(); // 所有的文件系统信息
    mkdir("dev/", "misc");
    let _rtc = open_file("./dev/misc/", "rtc", OpenFlags::CREATE).unwrap(); // 硬件时钟信息
    let _lat_sig = open_file(ROOT_DIR, "lat_sig", OpenFlags::CREATE).unwrap(); // lat_sig prot 测例要求的文件。测例只管读这个文件，但又不创建
}

/// 在 path 后加入 child_path 路径，返回 child_path 中最后一个 '/' 的位置+1。(如没有 '/' 则返回0)
/// 注意根路径的父路径视为自己。
///
/// 要求 path 必须以 './' 开头，以 '/' 结尾。
///
/// 因为 child_path 中可能包含 ./ 和 ../ ，所以可能处理后的 path 比输入时更短
fn parse_dir(path: &mut String, child_path: &str) -> usize {
    let mut pos = 0;
    loop {
        //println!("path = {}, child_path = {}, pos = {}", path.as_str(), child_path, pos);
        if let Some(new_pos) = (&child_path[pos..]).find('/') {
            // 这里用 new_pos == 2 是为了使用短路运算符，如果长度不是2就不需要做字符串比较了
            if new_pos == 2 && &child_path[pos..pos + new_pos] == ".." {
                // 删除一个 '/'
                path.pop();
                // 删除上一级目录，直到遇到根目录或者上一个 '/'
                while path.len() > 1 && path.pop() != Some('/') {}
                // 再加回 '/'
                path.push('/');
            } else if new_pos == 1 && &child_path[pos..pos + new_pos] == "." {
            } else if new_pos != 0 {
                // 加路径的时候要把 '/' 也加上
                *path += &child_path[pos..=pos + new_pos];
            }
            pos += new_pos + 1;
        } else {
            break pos;
        }
    }
}

/// 分割文件所在路径。返回的第一个值是路径，它是新生成的String，复制了需要的字符；第二个值是 slice，指向 file_path 的一部分。
///
/// 返回的 String 本质上是把 file_path 中除了文件名之外的部分取出，连在 dir_name 后
///
/// 函数会处理 "./" 和 "../"。如果地址不合法，或者 dir_name 没有以 '/' 结尾，则返回 None
fn split_path_and_file<'a>(dir_name: &str, file_path: &'a str) -> Option<(String, &'a str)> {
    if !dir_name.ends_with('/') {
        return None;
    }
    let mut dir = String::from("./");
    let start_pos = if dir_name.starts_with("./") {
        //一般来说，根目录是从 ./ 开始，所以 dir_name 也是 ./ 开头
        2
    } else if dir_name.starts_with("/") {
        // 但如果用户通过 getcwd 等方式获取目录，则这样的目录是以 / 开头的
        1
    } else {
        //又或者用户试图输入一个相对路径，这时需要把它变成相对于根路径的路径
        0
    };
    parse_dir(&mut dir, &dir_name[start_pos..]);
    let pos = parse_dir(&mut dir, file_path);
    Some((dir, &file_path[pos..]))
}

/// 分割文件所在路径，然后经过link转换。
fn map_path_and_file(dir_name: &str, file_path: &str) -> Option<(String, String)> {
    if !dir_name.ends_with('/') {
        return None;
    }
    let mut dir = String::from("./");
    let start_pos = if dir_name.starts_with("./") {
        //一般来说，根目录是从 ./ 开始，所以 dir_name 也是 ./ 开头
        2
    } else if dir_name.starts_with("/") {
        // 但如果用户通过 getcwd 等方式获取目录，则这样的目录是以 / 开头的
        1
    } else {
        //又或者用户试图输入一个相对路径，这时需要把它变成相对于根路径的路径
        0
    };
    parse_dir(&mut dir, &dir_name[start_pos..]);
    let pos = parse_dir(&mut dir, file_path);
    Some(parse_file_name((dir, String::from(&file_path[pos..]))))
}

/*
/// 获取文件所在路径。
fn get_file_dir<'a>(dir_name: &str, file_path: &'a str) -> String {
    map_path_and_file(dir_name, file_path).unwrap().0
}
*/

/// 打开目录。如果是根目录，特判直接返回 root；否则打开代表目录的 FsDir
///
/// 因为需要通过 move 传入 root，这个函数只在模块内使用。
/// 如果其他库需要打开目录(作为文件)，需要用 open_file 然后在 flags 里加入 DIR 一项
fn inner_open_dir(root: FsDir, dir_name: &str) -> Option<FsDir> {
    if dir_name == ROOT_DIR {
        Some(root)
    } else {
        // 根目录是 "./" ，所以所有目录也是以 "./" 开头的，这里输入 fatfs 时要过滤掉这两个字符
        if let Ok(dir) = root.open_dir(&dir_name[2..]) {
            Some(dir)
        } else {
            None
        }
    }
}

/// 在 dir_name 目录下，打开 name 文件。
/// 如果不包含 OpenFlags::DIR，可能出现如下情况：
///
/// 1. 文件存在，但要求创建 -> 清空文件并返回
/// 2. 文件存在，不要求创建 -> 直接返回文件
/// 3. 文件不存在，要求创建 -> 创建新文件并返回
/// 4. 文件不存在，不要求创建 -> 打开失败
/// 5. 文件不存在，但存在同名目录 -> 打开失败
/// 6. 其他情况，如路径不存在 -> 打开失败
///
/// 如果包含 OpenFlags::DIR，则只有打开已存在的目录成功时返回 FdDir
pub fn open_file(dir_name: &str, file_path: &str, flags: OpenFlags) -> Option<Arc<dyn File>> {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    //println!("dir_name {}, file_path {}", dir_name, file_path);
    let (real_dir, file_name) = map_path_and_file(dir_name, file_path)?;
    info!("real_dir {}, file_name {}", real_dir, file_name);
    // 先查询在 vfs 里是否有对应定义的文件
    let find_in_vfs = get_virt_file_if_possible(&real_dir, &file_name, flags);
    if find_in_vfs.is_some() {
        return find_in_vfs;
    }
    let file_name = if file_name == "." {
        &""
    } else {
        file_name.as_str()
    };
    //println!("dir = {}, name = {}, name_len {}", real_dir, file_name, file_name.len());
    if let Some(dir) = inner_open_dir(root, real_dir.as_str()) {
        if flags.contains(OpenFlags::DIR) || flags.contains(OpenFlags::DSYNC) || file_name.len() == 0 {
            // 要求打开目录
            // 用户传入 sys_open 的目录名如果是有斜线的，那么 file_path 就是空的了
            // 否则 file_path 是当前目录下的一个子目录的名字
            let dir = if file_name.len() == 0 {
                Ok(dir)
            } else {
                dir.open_dir(file_name)
            };
            match dir {
                Ok(_dir) => {
                    // 不考虑是否有 CREATE 参数，只要找到目录就可以直接返回
                    Some(Arc::new(FdDir::new(String::from(real_dir) + file_name)))
                }
                // 如果找不到，也不考虑 CREATE。创建目录应该用 mkdir 而不是 open_file
                _ => None,
            }
        } else {
            // 否则要求打开文件
            let (readable, writable) = flags.read_write();
            //println!("opened {}, {}", readable, writable);
            match dir.open_file(file_name) {
                Ok(file) => {
                    if flags.contains(OpenFlags::EXCL) {
                        //选项要求必须要创建文件
                        None
                    } else {
                        let fat_file = FatFile::new(
                            readable,
                            writable,
                            real_dir,
                            String::from(file_name),
                            file,
                            flags,
                        );
                        if flags.contains(OpenFlags::CREATE) {
                            // 清空这个文件
                            fat_file.file.lock().truncate().unwrap();
                        };
                        Some(Arc::new(fat_file))
                    }
                }
                Err(Error::NotFound) => {
                    if flags.contains(OpenFlags::CREATE) {
                        let file = dir.create_file(file_name).unwrap();
                        Some(Arc::new(FatFile::new(
                            readable,
                            writable,
                            real_dir,
                            String::from(file_name),
                            file,
                            flags,
                        )))
                    } else {
                        None
                    }
                }
                // 其他情况下(包括存在同名的目录的情况)，返回None
                _ => None,
            }
        }
    } else {
        None
    }
}

/// 检查文件是否存在。（不考虑link）
/// 如果目录本身不存在，那么也会返回 false，不会报错。
///
/// 这里并不直接试图打开文件检查是否成功，而是检查目录下是否存在对应文件。
/// 这是因为其他进程占用文件等情况也可能导致打开文件失败，所以打开失败不等于文件不存在
pub fn check_file_exists(dir_name: &str, file_path: &str) -> bool {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    map_path_and_file(dir_name, file_path)
        .map(|(real_dir, file_name)| {
            info!(
                "check file exists: dir = {}, name = {}",
                real_dir, file_name
            );
            if let Some(exist) = check_virt_file_exists(&real_dir, &file_name) {
                return exist;
            }
            inner_open_dir(root, real_dir.as_str())
                .map(|dir| {
                    for entry in dir.iter() {
                        let file = entry.unwrap();
                        if file.file_name() == file_name {
                            return !file.is_dir();
                        }
                    }
                    false
                })
                .map_or(false, |r| r)
        })
        .map_or(false, |r| r)
}

/// 删除文件
///
/// **调用这个函数时默认文件存在，且 path/name 已经过 split_path_and_file 格式化**
fn remove_file(path: &str, name: &str) {
    let root = MEMORY_FS.root_dir();
    // 如果在 vfs 里能找到文件，就直接在里面删除
    if let Some(_) = try_remove_virt_file(&path.into(), &name.into()) {
        return;
    }
    let dir = inner_open_dir(root, path).unwrap();
    dir.remove(name).unwrap();
    /*
    dir.remove(name).unwrap_or_else(|_| {
        println!("path [{}] name [{}]", path, name);
    });
    */
}

/// 创建目录，返回是否成功
pub fn mkdir(dir_name: &str, file_path: &str) -> bool {
    let root = MEMORY_FS.root_dir();
    map_path_and_file(dir_name, file_path)
        .map(|(real_dir, file_name)| {
            if let Some(vdir) = get_virt_dir_if_possible(&real_dir) {
                return try_make_virt_dir(&vdir, &file_name);
            }
            inner_open_dir(root, real_dir.as_str())
                .map(|dir| {
                    // 说明现在打开的 dir 就是想要创建的目录，那么它已经存在了
                    if file_name.len() == 0 {
                        return false;
                    }
                    // 检查目录或者同名文件是否已存在
                    for entry in dir.iter() {
                        let file = entry.unwrap();
                        if file.file_name() == file_name {
                            return false;
                        }
                    }
                    dir.create_dir(file_name.as_str()).is_ok()
                })
                .map_or(false, |r| r)
        })
        .map_or(false, |r| r)
}

/// 检查目录是否存在
/// 要求 dir_name 使用 os 中的格式，即以 "./" 开头
pub fn check_dir_exists(dir_name: &str) -> bool {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    let mut dir_name = String::from(dir_name);
    if !dir_name.ends_with('/') {
        dir_name.push('/');
    }
    // info!("dir is {}", dir_name);
    let dir_name = map_path_and_file(dir_name.as_str(), "").unwrap().0;
    if check_virt_dir_exists(&dir_name) == Some(true) {
        return true;
    }
    // 去掉字符串开头的 '.' 或者 "./"
    inner_open_dir(root, dir_name.as_str()).is_some()
}

/*
/// 获取目录下的第 entry_id 个 DirEntry，返回文件类型(是否是目录)以及文件名。如果找不到，返回 None
///
/// 这里实际上没有检查硬链接
pub fn get_kth_dir_entry_info_of_path(dir_name: &str, entry_id: usize) -> Option<(bool, String)> {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    let dir_name = map_path_and_file(dir_name, "").unwrap().0;
    info!("get dir: dir = {}", dir_name);
    inner_open_dir(root, dir_name.as_str())
        .map(|dir| {
            let mut now_id = 0;
            for entry in dir.iter() {
                if now_id == entry_id {
                    let file = entry.unwrap();
                    return Some((file.is_dir(), file.file_name()));
                }
                now_id += 1;
            }
            None
        })
        .unwrap_or(None)
}
*/

/// 获取一个迭代器，每次返回一个目录中的 DirEntry。如果对应目录不存在，返回 None
///
/// 这里实际上没有检查硬链接
pub fn get_dir_entry_iter<'a>(dir_name: &str) -> Option<FsDirIter<'a>> {
    //let fs = MEMORY_FS.lock();
    //let root = fs.root_dir();
    let root = MEMORY_FS.root_dir();
    let dir_name = map_path_and_file(dir_name, "").unwrap().0;
    info!("get dir: dir = {}", dir_name);
    inner_open_dir(root, dir_name.as_str()).map(|dir| {dir.iter()})
}

