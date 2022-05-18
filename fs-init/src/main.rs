use clap::{App, Arg, ArgMatches};
use std::env;
use std::fs::{self, File};

use fatfs::{
    format_volume, 
    FormatVolumeOptions, 
    StdIoWrapper, 
    FileSystem, 
    FsOptions
};
use fscommon::BufStream;
use chrono::{DateTime, Local};

// 初始化用户程序时，读执行此程序的计算机的fs中的文件，写入生成的 FAT-fs 中的文件
use std::io::{self, Read};
use fatfs::Write;

fn main() {
    //fs_test();
    pack_up_user_applications();
}

fn pack_up_user_applications() {
    let arguments = resolve_parser();
    let src_path = arguments.value_of("source").unwrap();
    let target_path = arguments.value_of("target").unwrap();
    let img_file = String::from(target_path) + "fat.img";
    println!("src_path = {}\ntarget_path = {}\nimg_file = {}", src_path, target_path, img_file);

    create_new_fs(img_file.as_str()).unwrap();
    let file = fs::OpenOptions::new().read(true).write(true).open(img_file.as_str()).unwrap();
    let buf_file = BufStream::new(file);
    let options = FsOptions::new().update_accessed_date(true);
    let fs = FileSystem::new(buf_file, options).unwrap();

    let root = fs.root_dir();
    let user_apps: Vec<_> = fs::read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in user_apps {
        let mut origin_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        origin_file.read_to_end(&mut all_data).unwrap();
        println!("app_name {}", app.as_str());
        let mut file_in_fs = root.create_file(app.as_str()).unwrap();
        file_in_fs.write_all(all_data.as_slice()).unwrap();
    }

    for r in root.iter() {
        let e = r.unwrap();
        let modified = DateTime::<Local>::from(e.modified())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        println!("{:4}  {}  {}", file_size_to_str(e.len()), modified, e.file_name());
    }
}

fn resolve_parser() -> ArgMatches<'static> {
    App::new("packer")
    .arg(
        Arg::with_name("source")
            .short("s")
            .long("source")
            .takes_value(true)
            .help("Executable source dir(with backslash)"),
    )
    .arg(
        Arg::with_name("target")
            .short("t")
            .long("target")
            .takes_value(true)
            .help("Executable target dir(with backslash)"),
    )
    .get_matches()
}

fn create_new_fs(name: &str) -> io::Result<()> {
    let img_file = fs::OpenOptions::new().read(true).write(true).create(true).open(&name).unwrap();
    img_file.set_len(16 * 2048 * 512).unwrap();
    let buf_file = BufStream::new(img_file);
    format_volume(&mut StdIoWrapper::from(buf_file), FormatVolumeOptions::new()).unwrap();
    Ok(())
}

fn file_size_to_str(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if size < KB {
        format!("{}B", size)
    } else if size < MB {
        format!("{}KB", size / KB)
    } else if size < GB {
        format!("{}MB", size / MB)
    } else {
        format!("{}GB", size / GB)
    }
}

#[test]
fn fs_test()  -> io::Result<()> {
    create_new_fs("fat.img")?;
    let img_file = fs::OpenOptions::new().read(true).write(true).open("fat.img")?;
    let buf_stream = BufStream::new(img_file);
    let fs = fatfs::FileSystem::new(buf_stream, fatfs::FsOptions::new())?;
    let root_dir = fs.root_dir();
    let mut file = root_dir.create_file("hello.txt")?;
    file.write_all(b"Hello World!")?;
    Ok(())
}

/*

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * 512).unwrap();
        f
    })));
    // 16MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file, 16 * 2048, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let apps: Vec<_> = read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {
        // load app data from host file system
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let inode = root_inode.create(app.as_str()).unwrap();
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    // list apps
    for app in root_inode.ls() {
        println!("{}", app);
    }
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(block_file.clone(), 4096, 1);
    let efs = EasyFileSystem::open(block_file.clone());
    let root_inode = EasyFileSystem::root_inode(&efs);
    root_inode.create("filea");
    root_inode.create("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, world!";
    filea.write_at(0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    let mut buffer = [0u8; 233];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);

    let mut random_str_test = |len: usize| {
        filea.clear();
        assert_eq!(filea.read_at(0, &mut buffer), 0,);
        let mut str = String::new();
        use rand;
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
        }
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SZ);
    random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    random_str_test(100 * BLOCK_SZ);
    random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    random_str_test((12 + 128) * BLOCK_SZ);
    random_str_test(400 * BLOCK_SZ);
    random_str_test(1000 * BLOCK_SZ);
    random_str_test(2000 * BLOCK_SZ);

    Ok(())
}
*/