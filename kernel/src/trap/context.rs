use riscv::register::sstatus::{self, Sstatus, SPP};
/// Trap Context
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapContext {
    /// general regs[0..31]
    pub x: [usize; 32],
    /// CSR sstatus      
    pub sstatus: Sstatus,
    /// CSR sepc
    pub sepc: usize,
}

impl TrapContext {
    /// 设置 sp 寄存器
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// 设置 a0 寄存器
    pub fn set_a0(&mut self, a0: usize) {
        self.x[10] = a0;
    }
    /// init app context
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        println!("init app entry {:x} sp {:x}", entry, sp);
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); //previous privilege mode: user mode
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // entry point of app
        };
        cx.set_sp(sp); // app's user stack pointer
        cx // return initial Trap Context of app
    }
    /// 空的 TrapContext
    pub fn new() -> Self {
        Self {
            x: [0; 32],
            sstatus: sstatus::read(),
            sepc: 0,
        }
    }
    
}
