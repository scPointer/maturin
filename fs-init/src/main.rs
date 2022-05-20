//! 初始化文件系统，将测例读入文件系统中。
//! 
//! 关于调用参数的用法详见 ../kernel/Makefile

#![deny(missing_docs)]

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

/// 生成文件系统镜像，导入用户程序
fn pack_up_user_applications() {
    let arguments = resolve_parser();
    let src_path = arguments.value_of("source").unwrap();
    let target_path = arguments.value_of("target").unwrap();
    let output_path = arguments.value_of("output").unwrap();
    let img_file = String::from(output_path) + "fat.img";
    println!("src_path = {}\ntarget_path = {}\noutput_path = {}\nimg_file = {}", src_path, target_path, output_path, img_file);
    let user_apps = if arguments.is_present("bin") {
        get_app_names_from_bin_dir(src_path)
    } else {
        get_app_names_from_code_dir(src_path)
    };

    create_new_fs(img_file.as_str()).unwrap();
    let file = fs::OpenOptions::new().read(true).write(true).open(img_file.as_str()).unwrap();
    let buf_file = BufStream::new(file);
    let options = FsOptions::new().update_accessed_date(true);
    let fs = FileSystem::new(buf_file, options).unwrap();
    let root = fs.root_dir();
    
    for app in user_apps {
        // 是子目录
        if app.ends_with("/") {
            println!("user dir: {}", app.as_str());
            root.create_dir(app.as_str()).unwrap();
        } else {
            //println!("{}", format!("{}{}", target_path, app));
            let mut origin_file = File::open(format!("{}{}", target_path, app)).unwrap();
            let mut all_data: Vec<u8> = Vec::new();
            origin_file.read_to_end(&mut all_data).unwrap();
            println!("user app: {}", app.as_str());
            let mut file_in_fs = root.create_file(app.as_str()).unwrap();
            file_in_fs.write_all(all_data.as_slice()).unwrap();
        }
        
    }

    for dir_entry in root.iter() {
        let file = dir_entry.unwrap();
        println!("{:4}  {}  {}", file_size_to_str(file.len()), date_time_to_str(file.modified()), file.file_name());
        // 如果是子目录，则再继续遍历
        if file.is_dir() {
            println!("{}/", file.file_name());
            for dir_entry in root.open_dir(file.file_name().as_str()).unwrap().iter() {
                let file = dir_entry.unwrap();
                // "." 开头的是当前目录、父目录以及(未来可能的)隐藏文件
                if !file.file_name().starts_with(".") {
                    println!("\t{:4}  {}  {}", file_size_to_str(file.len()), date_time_to_str(file.modified()), file.file_name());
                }
            }
        }
    }
}

/// 解析参数
fn resolve_parser() -> ArgMatches<'static> {
    App::new("packer")
    .arg( // 源文件目录
        Arg::with_name("source")
            .short("s")
            .long("source")
            .takes_value(true)
            .help("Executable source dir(with backslash)"),
    )
    .arg( // 二进制文件目录，是已编译链接好的用户程序
        Arg::with_name("target")
            .short("t")
            .long("target")
            .takes_value(true)
            .help("Executable target dir(with backslash)"),
    )
    .arg( // 输出目录，生成的镜像将存放在此处
        Arg::with_name("output")
            .short("o")
            .long("output")
            .takes_value(true)
            .help("Output File System Image dir(with backslash)"),
    )
    .arg( //是否是二进制文件。
        // 如果是，将输入视为二进制文件，在 source 下找对应的二进制文件
        // 否则，视为源文件，在 source 下找对应文件名，然后到 target 下找对应二进制文件
        Arg::with_name("bin")
            .short("b")
            .long("bin")
            .help("source is binary or not")
    )
    .get_matches()
}

/// 创建新的 FAT 格式文件系统镜像，存放在 name 指定的文件名(可能带路径)中
fn create_new_fs(name: &str) -> io::Result<()> {
    let img_file = fs::OpenOptions::new().read(true).write(true).create(true).open(&name).unwrap();
    img_file.set_len(16 * 2048 * 512).unwrap();
    let buf_file = BufStream::new(img_file);
    format_volume(&mut StdIoWrapper::from(buf_file), FormatVolumeOptions::new()).unwrap();
    Ok(())
}

/// 粗略显示文件大小
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

/// 显示创建时间
fn date_time_to_str(date_time: fatfs::DateTime) -> String {
    DateTime::<Local>::from(date_time)
    .format("%Y-%m-%d %H:%M:%S")
    .to_string()
}

/// 从源代码目录中读取每个用户程序的名字。
/// 默认用户程序只在根目录下
fn get_app_names_from_code_dir(path: &str) -> Vec<String> {
    fs::read_dir(path)
    .unwrap()
    .into_iter()
    .map(|dir_entry| {
        let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
        name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
        name_with_ext
    })
    .collect()
}

/// 从已编译好的用户程序的目录中读取每个用户程序的名字。
/// **默认最多只有一层目录**
/// 
/// 因为可能有目录，所以每次拿到的 DirEntry 可能内含多个文件，所以就不能 map-collect 了
fn get_app_names_from_bin_dir(path: &str) -> Vec<String> {
    let mut names: Vec<String> = vec![];
    for dir_entry in fs::read_dir(path).unwrap() {
        let file = dir_entry.unwrap();
        if file.path().is_dir() {
            println!("dir: {}",file.file_name().into_string().unwrap());
            let dir_name = file.file_name().into_string().unwrap();
            names.push(format!("{}/", dir_name));
            for inner_entry in fs::read_dir(file.path()).unwrap() {
                let inner_file = inner_entry.unwrap();
                // 略去第二层目录项。之后可以把这个函数改成递归的
                if !inner_file.path().is_dir() {
                    names.push(format!("{}/{}", dir_name, inner_file.file_name().into_string().unwrap()))
                }
            }
        } else {
            names.push(file.file_name().into_string().unwrap());
        }
    }
    names
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