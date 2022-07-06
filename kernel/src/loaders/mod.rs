mod abi;

use alloc::{string::String, sync::Arc, vec::Vec};
use core::convert::From;

use crate::error::{OSError, OSResult};
use crate::memory::addr::{page_count, page_offset, VirtAddr};
use crate::memory::{PmArea, PmAreaLazy, VmArea};
use crate::memory::{PTEFlags, MemorySet};
use crate::constants::{
    PAGE_SIZE,
    USER_STACK_OFFSET,
    USER_STACK_SIZE,
    LIBC_SO_NAME,
    LIBC_SO_FILE,
    LIBC_SO_DIR,
};
use crate::file::{open_file, OpenFlags};
use crate::utils::raw_ptr_to_ref_str;

use lock::Mutex;
use xmas_elf::{
    header,
    symbol_table::{DynEntry64, Entry},
    program::{Flags, SegmentData, Type},
    sections::SectionData,
    ElfFile,
};

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

        #[cfg(target_pointer_width = "32")]
        if elf.header.pt1.class() != header::Class::ThirtyTwo {
            return Err("64-bit ELF is not supported on the 32-bit system".into());
        }
        #[cfg(target_pointer_width = "64")]
        if elf.header.pt1.class() != header::Class::SixtyFour {
            return Err("32-bit ELF is not supported on the 64-bit system".into());
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
        if let Some(interp_header) = self.elf.program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Interp)) {
            let data = match interp_header.get_data(&self.elf).unwrap() {
                SegmentData::Undefined(data) => data,
                _ => return Err(OSError::Loader_InvalidSegment),
            };
            let path = unsafe { raw_ptr_to_ref_str(data.as_ptr()) };
            let path = if path == LIBC_SO_NAME {
                LIBC_SO_FILE
            } else {
                path
            };
            info!("path: {:?}", path);
            let mut new_args = vec![String::from(
                if path == LIBC_SO_NAME {
                    LIBC_SO_FILE
                } else {
                    path
                })];
            new_args.extend(args);
            info!("args {:#?}", new_args);
            return parse_user_app(LIBC_SO_DIR, LIBC_SO_FILE, vm, new_args);
        }

        let base = 0x30_0000;
        // push ELF segments to `vm`
        let mut elf_base_vaddr = 0;
        for ph in self.elf.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            //println!("page at {:x}, page to {:x}", ph.virtual_addr() as usize, (ph.virtual_addr() + ph.mem_size()) as VirtAddr);
            //println!("ph offset {:x}, ph le {:x}", ph.offset() as usize, ph.file_size() as usize);
            
            let pgoff = page_offset(ph.virtual_addr() as usize);
            let page_count = page_count(ph.mem_size() as usize + pgoff);
            let mut pma = PmAreaLazy::new(page_count)?;
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
            */
            //println!("creating MemorySet from ELF");
            pma.write(pgoff, data)?;
            let seg = VmArea::new(
                ph.virtual_addr() as VirtAddr + base,
                (ph.virtual_addr() + ph.mem_size()) as VirtAddr + base,
                ph.flags().into(),
                Arc::new(Mutex::new(pma)),
                "elf_segment",
            )?;
            //println!("{:#?}", seg);
            vm.push(seg)?;
            if ph.offset() == 0 {
                elf_base_vaddr = ph.virtual_addr() as usize;
            }
        }

        // 如果需要重定位，即这是动态执行程序
        if let Some(rela_header) = self.elf.find_section_by_name(".rela.dyn") {
            let data = match rela_header.get_data(&self.elf).unwrap() {
                SectionData::Rela64(data) => data,
                _ => return Err(OSError::Loader_InvalidSection),
            };
            // 有 .rela.dyn 就应该对应有 .dynsym，所以不再用 if let 检查，直接上问号表达式
            let dynamic_symbols = match self.elf.find_section_by_name(".dynsym")
                .ok_or(OSError::Loader_InvalidSection)?
                .get_data(&self.elf)
                .unwrap() {
                    SectionData::DynSymbolTable64(dsym) => dsym,
                    _ => return Err(OSError::Loader_InvalidSection),
                };
            for entry in data.iter() {
                match entry.get_type() {
                    abi::REL_GOT | abi::REL_PLT | abi::R_RISCV_64 => {
                        let dynsym = &dynamic_symbols[entry.get_symbol_table_index() as usize];
                        let symval = if dynsym.shndx() == 0 {
                            let name = dynsym.get_name(&self.elf)?;
                            panic!("need to find symbol: {:?}", name);
                        } else {
                            base + dynsym.value() as usize
                        };
                        let value = symval + entry.get_addend() as usize;
                        let addr = base + entry.get_offset() as usize;
                        info!("GOT write: {:#x} @ {:#x}", value, addr);
                        vm.write(addr, core::mem::size_of::<usize>(), &value.to_ne_bytes(), PTEFlags::empty())?;
                        //vmar.write_memory(addr, &value.to_ne_bytes()).map_err(|_| "Invalid Vmar")?;
                    }
                    abi::REL_RELATIVE | abi::R_RISCV_RELATIVE => {
                        let value = base + entry.get_addend() as usize;
                        let addr = base + entry.get_offset() as usize;
                        info!("RELATIVE write: {:#x} @ {:#x}", value, addr);
                        vm.write(addr, core::mem::size_of::<usize>(), &value.to_ne_bytes(), PTEFlags::empty())?;
                    }
                    t => unimplemented!("unknown type: {}", t),
                }
            }
        }

        if let Some(rela_header) = self.elf.find_section_by_name(".rela.plt") {
            let data = match rela_header.get_data(&self.elf).unwrap() {
                SectionData::Rela64(data) => data,
                _ => return Err(OSError::Loader_InvalidSection),
            };
            let dynamic_symbols = match self.elf.find_section_by_name(".dynsym")
                .ok_or(OSError::Loader_InvalidSection)?
                .get_data(&self.elf)
                .unwrap() {
                    SectionData::DynSymbolTable64(dsym) => dsym,
                    _ => return Err(OSError::Loader_InvalidSection),
                };
            for entry in data.iter() {
                //info!("here");
                match entry.get_type() {
                    5 => {
                        let dynsym = &dynamic_symbols[entry.get_symbol_table_index() as usize];
                        let symval = if dynsym.shndx() == 0 {
                            let name = dynsym.get_name(&self.elf)?;
                            panic!("need to find symbol: {:?}", name);
                        } else {
                            dynsym.value() as usize
                        };
                        let value = base + symval;
                        let addr = base + entry.get_offset() as usize;
                        //info!("type = {} write: {:#x} @ {:#x}", entry.get_type() as usize, value, addr);
                        vm.write(addr, core::mem::size_of::<usize>(), &value.to_ne_bytes(), PTEFlags::empty())?;
                        //vmar.write_memory(addr, &value.to_ne_bytes()).map_err(|_| "Invalid Vmar")?;
                    }
                    t => unimplemented!("unknown type: {}", t),
                }
            }
        }

        let entry = self.elf.header.pt2.entry_point() as usize;
        let stack_bottom = USER_STACK_OFFSET;
        let mut stack_top = stack_bottom + USER_STACK_SIZE;
        let mut stack_pma = PmAreaLazy::new(page_count(USER_STACK_SIZE))?;

        // push `ProcInitInfo` to user stack
        let info = abi::ProcInitInfo {
            args,
            envs: Vec::new(),
            auxv: {
                use alloc::collections::btree_map::BTreeMap;
                let mut map = BTreeMap::new();
                map.insert(abi::AT_BASE, elf_base_vaddr + base);
                map.insert(
                    abi::AT_PHDR,
                    elf_base_vaddr + base + self.elf.header.pt2.ph_offset() as usize,
                );
                map.insert(abi::AT_ENTRY, entry);
                map.insert(abi::AT_PHENT, self.elf.header.pt2.ph_entry_size() as usize);
                map.insert(abi::AT_PHNUM, self.elf.header.pt2.ph_count() as usize);
                map.insert(abi::AT_PAGESZ, PAGE_SIZE);
                map
            },
        };
        let init_stack = info.push_at(stack_top);
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
        Ok((entry + base, stack_top))
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
/// 根据名字获取二进制串形式的用户程序，如找不到，则返回某种 OSError
/// 
pub fn parse_user_app(
    app_dir: &str,
    app_name: &str, 
    mut vm: &mut MemorySet, 
    args: Vec<String>
) -> OSResult<(VirtAddr, VirtAddr)> {
    open_file(app_dir, app_name, OpenFlags::RDONLY)
        .map(|node| unsafe { node.read_all() } )
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
