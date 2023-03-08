bitflags::bitflags! {
    /// 页表项各位的定义。目前对应 riscv64 的 Sv39 模式
    pub struct PTEFlags: usize {
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

impl Into<usize> for PTEFlags {
    fn into(self) -> usize {
        self.bits
    }
}

impl From<usize> for PTEFlags {
    fn from(bits: usize) -> Self {
        PTEFlags { bits }
    }
}

#[allow(non_snake_case)]
pub fn PTE_NORMAL() -> PTEFlags {
    PTEFlags::VALID | PTEFlags::ACCESS | PTEFlags::DIRTY
}

#[allow(non_snake_case)]
pub fn PTE_U() -> PTEFlags {
    PTE_NORMAL() | PTEFlags::USER
}

#[allow(non_snake_case)]
pub fn PTE_RU() -> PTEFlags {
    PTE_U() | PTEFlags::READ
}

#[allow(non_snake_case)]
pub fn PTE_RWU() -> PTEFlags {
    PTE_RU() | PTEFlags::WRITE
}

#[allow(non_snake_case)]
pub fn PTE_RXU() -> PTEFlags {
    PTE_RU() | PTEFlags::EXECUTE
}
