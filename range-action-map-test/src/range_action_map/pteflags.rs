bitflags::bitflags! {
    /// 页表项各位的定义。目前对应 riscv64 的 Sv39 模式
    pub struct PTEFlags: u8 {
        const VALID = 1 << 0;
        const READ = 1 << 1;
        const WRITE = 1 << 2;
        const EXECUTE = 1 << 3;
        const USER = 1 << 4;
        const GLOBAL = 1 << 5;
        const ACCESS = 1 << 6;
        const DIRTY = 1 << 7;
    }
}

pub fn PTE_NORMAL() -> PTEFlags {
    PTEFlags::VALID | PTEFlags::ACCESS | PTEFlags::DIRTY
}

pub fn PTE_U() -> PTEFlags {
    PTE_NORMAL() | PTEFlags::USER
}

pub fn PTE_RU() -> PTEFlags {
    PTE_U() | PTEFlags::READ
}

pub fn PTE_RWU() -> PTEFlags {
    PTE_RU() | PTEFlags::WRITE
}

pub fn PTE_RXU() -> PTEFlags {
    PTE_RU() | PTEFlags::EXECUTE
}
