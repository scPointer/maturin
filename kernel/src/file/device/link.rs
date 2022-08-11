//! 处理文件系统的链接相关
//!
//! 这个模块中有大量字符串操作，可能有较高的时间复杂度，不建议频繁链接

//#![deny(missing_docs)]

use super::{check_dir_exists, check_file_exists, remove_file, split_path_and_file};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use lock::Mutex;

/// 用户看到的文件到实际文件的映射
static LINK_PATH_MAP: Mutex<BTreeMap<FileDisc, FileDisc>> = Mutex::new(BTreeMap::new());
/// 实际文件(而不是用户文件)到链接数的映射
static LINK_COUNT_MAP: Mutex<BTreeMap<FileDisc, usize>> = Mutex::new(BTreeMap::new());

/// 将用户提供的路径和文件转换成实际的路径和文件
pub fn parse_file_name((path, file): (String, String)) -> (String, String) {
    //info!("parse {} {}", path, file);
    let map = LINK_PATH_MAP.lock();
    match map.get(&FileDisc::new(&path, &file)) {
        Some(disc) => (String::from(&disc.path[..]), String::from(&disc.file[..])),
        None => (path, file),
    }
    //*count.entry(x).or_insert(0) += 1;
}

/// 添加硬链接
///
/// 这个函数不对外可见，外部需要调用 try_add_link
fn add_link(real_path: String, real_file: String, user_path: String, user_file: String) {
    //info!("add link {} {} {} {}", real_path, real_file, user_path, user_file);
    let mut map = LINK_PATH_MAP.lock();
    let mut count_map = LINK_COUNT_MAP.lock();
    let key = FileDisc::new(&user_path, &user_file);
    let value = FileDisc::new(&real_path, &real_file);
    // 注意链接数是统计在实际文件上的
    *count_map.entry(value.clone()).or_insert(1) += 1;
    match map.get(&key) {
        Some(_disc) => {
            map.insert(key, value);
        }
        None => {
            map.insert(key.clone(), value.clone());
            // 原来的文件自己也是一个链接，两者需要无法区分
            map.insert(value.clone(), value.clone());
        }
    };
}

/// 尝试添加一个硬链接。
///
/// 如果需要链接的文件已存在，或者被链接到的文件不存在，则执行失败，返回 false
pub fn try_add_link(old_path: String, old_file: &str, new_path: String, new_file: &str) -> bool {
    // 经过链接转换
    if let Some((old_path, old_file)) = split_path_and_file(old_path.as_str(), old_file)
        .map(|(path, file)| (path, String::from(file)))
        .map(parse_file_name)
    {
        if let Some((new_path, new_file)) = split_path_and_file(new_path.as_str(), new_file)
            .map(|(path, file)| (path, String::from(file)))
            .map(parse_file_name)
        {
            if check_file_exists(old_path.as_str(), old_file.as_str())
                && !check_file_exists(new_path.as_str(), new_file.as_str())
            {
                add_link(old_path, old_file, new_path, new_file);
                return true;
            }
        }
    }
    false
}

/// 获取硬链接数。
///
/// **默认该文件存在，且目录/文件格式经过split_path_and_file 转换**
pub fn get_link_count(path: String, file: &str) -> usize {
    let (path, file) = parse_file_name((path, String::from(file)));
    // 注意找不到时，链接数默认为 1 而不是 0。因为没有进行过链接操作的文件不在 map 里
    *LINK_COUNT_MAP
        .lock()
        .get(&FileDisc::new(&path, &file))
        .unwrap_or(&1)
}

/// 尝试删除一个硬链接。
/// 如果链接数为0，则删除该文件。
///
/// 如果这个文件不存在，则执行失败，返回 false
pub fn try_remove_link(path: String, file: &str) -> bool {
    let key = FileDisc::new(&path, &String::from(file));
    // 经过链接转换
    if let Some((real_path, real_file)) = split_path_and_file(path.as_str(), file)
        .map(|(path, file)| (path, String::from(file)))
        .map(parse_file_name)
    {
        if check_file_exists(real_path.as_str(), real_file.as_str()) {
            let mut map = LINK_PATH_MAP.lock();
            let mut count_map = LINK_COUNT_MAP.lock();
            let value = FileDisc::new(&real_path, &real_file);
            // 先删除链接表里的映射
            if count_map.get(&value).is_some() {
                map.remove(&key).unwrap();
            }
            // 链接表里没找到时，视作链接数为1
            let count = count_map.entry(value.clone()).or_insert(1);
            *count -= 1;
            // 如果已经没有链接了，则需要删除这个文件
            if *count == 0 {
                count_map.remove(&value).unwrap();
                info!("file removed.");
                remove_file(real_path.as_str(), real_file.as_str());
            }
            return true;
        } else if check_dir_exists(&[real_path.as_str(), real_file.as_str()].concat()) {
            // 目录则直接删除，因为目录不能链接，所以不需要处理链接表
            remove_file(real_path.as_str(), real_file.as_str());
            return true;
        }
    }
    false
}

/// 同时保存文件路径和文件名，作为链接表的 K/V
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct FileDisc {
    pub path: String,
    pub file: String,
}

impl FileDisc {
    pub fn new(path: &String, file: &String) -> Self {
        Self {
            path: String::from(&path[..]),
            file: String::from(&file[..]),
        }
    }
}

/// 挂载的文件系统。
/// 目前"挂载"的语义是，把一个文件当作文件系统读写
/// TODO: 把 mod.rs 中文件系统的操作全部封装为 struct，然后挂载时用文件实例化它
pub struct MountedFs {
    //pub inner: Arc<Mutex<FATFileSystem>>,
    pub device: String,
    pub mnt_dir: String,
}

impl MountedFs {
    pub fn new(device: &str, mnt_dir: &str) -> Self {
        Self {
            //inner: Arc::new_uninit(),
            device: String::from(device),
            mnt_dir: String::from(mnt_dir),
        }
    }
}

/// 已挂载的文件系统(设备)。
/// 注意启动时的文件系统不在这个 vec 里，它在 mod.rs 里。
static MOUNTED: Mutex<Vec<MountedFs>> = Mutex::new(Vec::new());

/// 挂载一个fatfs类型的设备
pub fn mount_fat_fs(device_path: String, device_file: &str, mount_path: String) -> bool {
    // 地址经过链接转换
    if let Some((device_path, device_file)) = split_path_and_file(device_path.as_str(), device_file)
        .map(|(path, file)| (path, String::from(file)))
        .map(parse_file_name)
    {
        let mount_path = split_path_and_file(mount_path.as_str(), "").unwrap().0;
        // mount_path 不需要转换，因为目前目录没有链接。只需要检查其在挂在前是否存在
        if check_dir_exists(mount_path.as_str())
        // && check_file_exists(device_path.as_str(), device_file.as_str())
        {
            MOUNTED.lock().push(MountedFs::new(
                (device_path + device_file.as_str()).as_str(),
                mount_path.as_str(),
            ));
            return true;
        }
    }
    false
}

pub fn umount_fat_fs(mount_path: String) -> bool {
    let mount_path = split_path_and_file(mount_path.as_str(), "").unwrap().0;
    let mut mounted = MOUNTED.lock();
    let size_before = mounted.len();
    mounted.retain(|mfs| mfs.mnt_dir != mount_path);
    mounted.len() < size_before
}
