mod abi;

use alloc::{string::String, sync::Arc, vec::Vec};
use core::convert::From;

use crate::error::{OSError, OSResult};
use crate::memory::addr::{page_count, page_offset, VirtAddr};
use crate::memory::{PmArea, PmAreaLazy, VmArea};
use crate::memory::{PTEFlags, MemorySet};
use crate::constants::{PAGE_SIZE, USER_STACK_OFFSET, USER_STACK_SIZE};

use lock::mutex::Mutex;
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
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

        if elf.header.pt2.type_().as_type() != header::Type::Executable {
            return Err("ELF is not executable object".into());
        }
        match elf.header.pt2.machine().as_machine() {
            #[cfg(target_arch = "riscv64")]
            header::Machine::Other(0xF3) => {}
            _ => return Err("invalid ELF arch".into()),
        };
        Ok(Self { elf })
    }

    pub fn init_vm(
        &self,
        vm: &mut MemorySet,
        args: Vec<String>,
    ) -> OSResult<(VirtAddr, VirtAddr)> {
        println!("creating MemorySet from ELF...");

        // push ELF segments to `vm`
        let mut elf_base_vaddr = 0;
        for ph in self.elf.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            println!("page at {:x}, page to {:x}", ph.virtual_addr() as usize, (ph.virtual_addr() + ph.mem_size()) as VirtAddr);
            let pgoff = page_offset(ph.virtual_addr() as usize);
            let page_count = page_count(ph.mem_size() as usize + pgoff);
            let mut pma = PmAreaLazy::new(page_count)?;
            let data = match ph.get_data(&self.elf).unwrap() {
                SegmentData::Undefined(data) => data,
                _ => return Err(OSError::Loader_InvalidSegment),
            };
            pma.write(pgoff, data)?;

            let seg = VmArea::new(
                ph.virtual_addr() as VirtAddr,
                (ph.virtual_addr() + ph.mem_size()) as VirtAddr,
                ph.flags().into(),
                Arc::new(Mutex::new(pma)),
                "elf_segment",
            )?;
            println!("{:#x?}", seg);
            vm.push(seg)?;
            if ph.offset() == 0 {
                elf_base_vaddr = ph.virtual_addr() as usize;
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
                map.insert(abi::AT_BASE, elf_base_vaddr);
                map.insert(
                    abi::AT_PHDR,
                    elf_base_vaddr + self.elf.header.pt2.ph_offset() as usize,
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

        Ok((entry, stack_top))
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
