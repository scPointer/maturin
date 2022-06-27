use std::fs::{read_dir, File};
use std::io::{Result, Write};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../fat.img");
    //println!("cargo:rerun-if-changed={}", TARGET_PATH);

    insert_fs_img().unwrap();
    //insert_app_data().unwrap();
}

//static TARGET_PATH: &str = "../user/target/riscv64imac-unknown-none-elf/release/";
static IMG_PATH: &str = "../fat.img";

fn insert_fs_img() -> Result<()> {
    let mut f = File::create("src/fs.S").unwrap();
    if !Path::new(IMG_PATH).exists() {
        return Ok(());
    }
    writeln!(
        f,
        r#"
    .section .data
    .global img_start
    .global img_end
    .align 12
img_start:
    .incbin "{}"
img_end:"#,
    IMG_PATH,
    )?;
    Ok(())
}
