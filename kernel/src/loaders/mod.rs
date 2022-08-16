mod flags;
use flags::*;
mod init_info;
use init_info::InitInfo;
mod init_stack;
use init_stack::InitStack;

use alloc::{string::String, sync::Arc, vec::Vec};
use core::convert::From;
use lock::Mutex;
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
    sections::SectionData,
    symbol_table::Entry,
    ElfFile,
};

use crate::constants::{
    //LIBC_SO_NAME,
    //LIBC_SO_FILE,
    //LIBC_SO_DIR,
    ELF_BASE_RELOCATE,
    PAGE_SIZE,
    ROOT_DIR,
    USER_STACK_OFFSET,
    USER_STACK_SIZE,
};
use crate::error::{OSError, OSResult};
use crate::file::{open_file, OpenFlags};
use crate::memory::addr::{page_count, page_offset, VirtAddr};
use crate::memory::{MemorySet, PTEFlags};
use crate::memory::{PmArea, PmAreaLazy, VmArea};
use crate::utils::raw_ptr_to_ref_str;

pub struct ElfLoader<'a> {
    elf: ElfFile<'a>,
}

impl From<&str> for OSError {
    fn from(s: &str) -> Self {
        println!("parse ELF file failed: {}", s);
        OSError::Loader_ParseElfFailed
    }
}

impl<'a> ElfLoader<'a> {
    pub fn new(elf_data: &'a [u8]) -> OSResult<Self> {
        let elf = ElfFile::new(elf_data).unwrap();
        // 检查类型
        if elf.header.pt1.class() != header::Class::SixtyFour {
            return Err("32-bit ELF is not supported on the riscv64".into());
        }
        /*
        if elf.header.pt2.type_().as_type() != header::Type::Executable {
            return Err("ELF is not executable object".into());
        }
        */
        match elf.header.pt2.machine().as_machine() {
            #[cfg(target_arch = "riscv64")]
            header::Machine::Other(0xF3) => {}
            _ => return Err("invalid ELF arch".into()),
        };
        Ok(Self { elf })
    }
    /// 解析 elf 文件并初始化一个用户程序，其中 args 为用户程序执行时的参数。
    ///
    /// 返回用户栈顶程序入口地址以及用户栈栈顶
    ///
    /// 这里会把 argc 存在用户栈顶， argv 存在栈上第二个元素，**且用 usize(64位) 存储**，即相当于：
    ///
    /// argc = *sp;
    ///
    /// argv = *(sp+4);
    pub fn init_vm(
        &mut self,
        vm: &mut MemorySet,
        args: Vec<String>,
    ) -> OSResult<(VirtAddr, VirtAddr)> {
        info!("creating MemorySet from ELF...");

        // 尝试获取 interpreter 段
        if let Some(interp_header) = self
            .elf
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Interp))
        {
            let data = match interp_header.get_data(&self.elf).unwrap() {
                SegmentData::Undefined(data) => data,
                _ => return Err(OSError::Loader_InvalidSegment),
            };
            let path = unsafe { raw_ptr_to_ref_str(data.as_ptr()) };
            info!("path: {:?}", path);
            let mut new_args = vec![String::from(path)];
            new_args.extend(args);
            info!("args {:#?}", new_args);
            return if let Some(pos) = path.rfind("/") {
                parse_user_app(&path[..=pos], &path[pos + 1..], vm, new_args)
            } else {
                parse_user_app(ROOT_DIR, path, vm, new_args)
            };
        }
        //println!("args {:#?}", args);
        // 动态程序在加载时用到的地址。如果是静态程序，则这里是 0
        let mut dyn_base = 0;
        // 先获取起始位置。
        // 虽然比较繁琐，但因为之后对 VmArea 的处理涉及这个基地址，所以需要提前获取
        let elf_base_vaddr = if let Some(header) = self
            .elf
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0)
        {
            // 找到第一段指示的地址
            let phdr = header.virtual_addr() as usize;
            info!("phdr = {:x}", phdr);
            // 如果是 0，如 libc.so，则需要放到一个非零的合法地址。此处规定从某个特定位置开始往后找。
            // 这样设置是因为，动态库运行时可能会mmap实际的用户程序且指定 MAP_FIXED，
            // 而用户程序的地址一般较低。为了让它们直接尽可能不冲突，所以会放到稍高的地址
            if phdr != 0 {
                phdr
            } else {
                dyn_base = ELF_BASE_RELOCATE;
                ELF_BASE_RELOCATE
            }
        } else {
            //return Err(OSError::Loader_PhdrNotFound);
            // 自行构造的测例(rcore/初赛)可能会出现这种情况，而且也没有 phdr 段，此时认为 base addr = 0
            0
        };

        for ph in self.elf.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            //println!("page at {:x}, page to {:x}", ph.virtual_addr() as usize, (ph.virtual_addr() + ph.mem_size()) as VirtAddr);
            //println!("ph offset {:x}, ph le {:x}", ph.offset() as usize, ph.file_size() as usize);

            let pgoff = page_offset(ph.virtual_addr() as usize);
            let page_count = page_count(ph.mem_size() as usize + pgoff);
            let mut pma = PmAreaLazy::new(page_count, None)?;
            //let data = &self.elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize];
            //let d0 = &self.elf.input;

            let data = match ph.get_data(&self.elf).unwrap() {
                SegmentData::Undefined(data) => data,
                _ => return Err(OSError::Loader_InvalidSegment),
            };
            //println!("data len {}", data.len());
            /*
            for i in 0..20 {
                print!("{} ", data[i]);
            }
            */
            /*
            for i in 0..20 {
                print!("{} ", d0[i]);
            }
            //println!("creating MemorySet from ELF");
            info!("ph virtual addr {:x} pgoff {:x}", ph.virtual_addr(), pgoff);
            if ph.virtual_addr() == 0xf51c0 {
                info!("data {:x}", data[0xfb300 - ph.virtual_addr() as usize]);
            }
            */
            pma.write(pgoff, data)?;
            let seg = VmArea::new(
                ph.virtual_addr() as VirtAddr + dyn_base,
                (ph.virtual_addr() + ph.mem_size()) as VirtAddr + dyn_base,
                ph.flags().into(),
                Arc::new(Mutex::new(pma)),
                "elf_segment",
            )?;
            //info!("{:#?}", seg);
            vm.push(seg)?;
        }

        // 如果需要重定位，即这是动态执行程序
        if let Some(rela_header) = self.elf.find_section_by_name(".rela.dyn") {
            let data = match rela_header.get_data(&self.elf).unwrap() {
                SectionData::Rela64(data) => data,
                _ => return Err(OSError::Loader_InvalidSection),
            };

            // 再检查是否有 .dynsym，如果没有说明应该是静态编译的，那么不处理 .rela.dyn
            if let Some(dynsym_header) = self.elf.find_section_by_name(".dynsym") {
                let dynamic_symbols = match dynsym_header.get_data(&self.elf).unwrap() {
                    SectionData::DynSymbolTable64(dsym) => dsym,
                    _ => return Err(OSError::Loader_InvalidSection),
                };
                for entry in data.iter() {
                    match entry.get_type() {
                        REL_GOT | REL_PLT | R_RISCV_64 => {
                            let dynsym = &dynamic_symbols[entry.get_symbol_table_index() as usize];
                            let symval = if dynsym.shndx() == 0 {
                                let name = dynsym.get_name(&self.elf)?;
                                panic!("symbol not found: {:?}", name);
                            } else {
                                dyn_base + dynsym.value() as usize
                            };
                            let value = symval + entry.get_addend() as usize;
                            let addr = dyn_base + entry.get_offset() as usize;
                            //info!("write: {:#x} @ {:#x} type = {}", value, addr, entry.get_type() as usize);
                            vm.write(
                                addr,
                                core::mem::size_of::<usize>(),
                                &value.to_ne_bytes(),
                                PTEFlags::empty(),
                            )?;
                            //vmar.write_memory(addr, &value.to_ne_bytes()).map_err(|_| "Invalid Vmar")?;
                        }
                        REL_RELATIVE | R_RISCV_RELATIVE => {
                            let value = dyn_base + entry.get_addend() as usize;
                            let addr = dyn_base + entry.get_offset() as usize;
                            //info!("write: {:#x} @ {:#x} type = {}", value, addr, entry.get_type() as usize);
                            vm.write(
                                addr,
                                core::mem::size_of::<usize>(),
                                &value.to_ne_bytes(),
                                PTEFlags::empty(),
                            )?;
                        }
                        t => panic!("[kernel] unknown entry, type = {}", t),
                    }
                }
            }
        }

        if let Some(rela_header) = self.elf.find_section_by_name(".rela.plt") {
            let data = match rela_header.get_data(&self.elf).unwrap() {
                SectionData::Rela64(data) => data,
                _ => return Err(OSError::Loader_InvalidSection),
            };
            let dynamic_symbols = match self
                .elf
                .find_section_by_name(".dynsym")
                .ok_or(OSError::Loader_InvalidSection)?
                .get_data(&self.elf)
                .unwrap()
            {
                SectionData::DynSymbolTable64(dsym) => dsym,
                _ => return Err(OSError::Loader_InvalidSection),
            };
            for entry in data.iter() {
                match entry.get_type() {
                    5 => {
                        let dynsym = &dynamic_symbols[entry.get_symbol_table_index() as usize];
                        let symval = if dynsym.shndx() == 0 {
                            let name = dynsym.get_name(&self.elf)?;
                            panic!("symbol not found: {:?}", name);
                        } else {
                            dynsym.value() as usize
                        };
                        let value = dyn_base + symval;
                        let addr = dyn_base + entry.get_offset() as usize;
                        //info!("write: {:#x} @ {:#x} type = {}", value, addr, entry.get_type() as usize);
                        vm.write(
                            addr,
                            core::mem::size_of::<usize>(),
                            &value.to_ne_bytes(),
                            PTEFlags::empty(),
                        )?;
                        //vmar.write_memory(addr, &value.to_ne_bytes()).map_err(|_| "Invalid Vmar")?;
                    }
                    t => panic!("[kernel] unknown entry, type = {}", t),
                }
            }
        }
        let user_entry = self.elf.header.pt2.entry_point() as usize;
        let stack_bottom = USER_STACK_OFFSET;
        let mut stack_top = stack_bottom + USER_STACK_SIZE;
        let mut stack_pma = PmAreaLazy::new(page_count(USER_STACK_SIZE), None)?;

        // push `ProcInitInfo` to user stack
        let info = InitInfo {
            args,
            envs: {
                vec![
                    "ENOUGH=50000".into(),
                    //"LMBENCH_SCHED=DEFAULT".into(),
                    //"TMPDIR=/tmp".into(),
                ]
            },
            auxv: {
                use alloc::collections::btree_map::BTreeMap;
                let mut map = BTreeMap::new();
                //map.insert(AT_BASE, elf_base_vaddr);
                map.insert(
                    AT_PHDR,
                    elf_base_vaddr + self.elf.header.pt2.ph_offset() as usize,
                );
                //map.insert(AT_ENTRY, user_entry);
                map.insert(AT_PHENT, self.elf.header.pt2.ph_entry_size() as usize);
                map.insert(AT_PHNUM, self.elf.header.pt2.ph_count() as usize);
                // AT_RANDOM 比较特殊，要求指向栈上的 16Byte 的随机子串。因此这里的 0 只是占位，在之后序列化时会特殊处理
                map.insert(AT_RANDOM, 0);
                map.insert(AT_PAGESZ, PAGE_SIZE);
                map
            },
        };
        let init_stack = info.serialize(stack_top);
        info!("stack len {}", init_stack.len());
        stack_pma.write(USER_STACK_SIZE - init_stack.len(), &init_stack)?;
        stack_top -= init_stack.len();

        // push user stack to `vm`
        let stack_vma = VmArea::new(
            stack_bottom,
            stack_top,
            PTEFlags::READ | PTEFlags::WRITE | PTEFlags::USER,
            Arc::new(Mutex::new(stack_pma)),
            "user_stack",
        )?;
        vm.push(stack_vma)?;
        // println!("{:#x?}", vm);
        Ok((user_entry + dyn_base, stack_top))
    }
}

impl From<Flags> for PTEFlags {
    fn from(f: Flags) -> Self {
        let mut ret = PTEFlags::USER;
        if f.is_read() {
            ret |= PTEFlags::READ;
        }
        if f.is_write() {
            ret |= PTEFlags::WRITE;
        }
        if f.is_execute() {
            ret |= PTEFlags::EXECUTE;
        }
        ret
    }
}

#[allow(unused)]
/// 执行用户程序并选择解释器：
/// - 如果程序以 .sh 结尾，则使用 busybox sh 执行
/// - 否则，将用户程序视为根据名字获取二进制串形式的用户程序
///
/// 如找不到，则返回某种 OSError
pub fn parse_user_app(
    app_dir: &str,
    app_name: &str,
    mut vm: &mut MemorySet,
    args: Vec<String>,
) -> OSResult<(VirtAddr, VirtAddr)> {
    let (app_dir, app_name, args) = if app_name.ends_with(".sh") {
        // .sh 文件统一用 busybox 解析
        (
            ROOT_DIR,
            "busybox",
            [
                vec![
                    String::from("busybox"),
                    String::from("sh"),
                    String::from(app_dir) + &args[0],
                ],
                Vec::from(&args[1..]),
            ]
            .concat(),
        )
    } else {
        (app_dir, app_name, args)
    };
    open_file(app_dir, app_name, OpenFlags::RDONLY)
        .map(|node| unsafe { node.read_all() })
        .map(|data| {
            /*
            for i in 0..20 {
                print!("{} ", data[i]);
            }
            */
            let mut loader = ElfLoader::new(data.as_slice())?;
            loader.init_vm(&mut vm, args)
        })
        .unwrap_or(Err(OSError::Loader_AppNotFound))
}
