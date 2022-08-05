use std::{
    env,
    fs::{self, File},
    io::{Result, Write},
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed=../fat.img");
    insert_fs_img().unwrap();

    let ld = &PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(ld, LINKER).unwrap();
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}

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

const IMG_PATH: &str = "../fat.img";

const LINKER: &str = "\
OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0xffffffff80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    .text : {
        stext = .;
        *(.text.entry)
        *(.text .text.*)
        etext = .;
    }

    . = ALIGN(4K);
    .rodata : {
        srodata = .;
        *(.rodata .rodata.*)
        erodata = .;
    }

    . = ALIGN(4K);
    .data : {
        sdata = .;
        *(.data .data.*)
        edata = .;
    }

    . = ALIGN(4K);
    sbss_with_stack = .;
    .bss : {
	    *(.bss.stack)
        sbss = .;
        *(.sbss .bss .bss.*)
        ebss = .;
    }

    . = ALIGN(4K);
    kernel_end = .;
    PROVIDE(end = .);
}
";
