use core::ptr::{null_mut, addr_of};
use core::arch::asm;
use riscv::register::{sie, sstatus, mstatus};
use crate::_stack_start;

pub const MAX_HARTS:usize = 4;

#[derive(Clone, Copy)]
pub struct TrapFrame{
    pub regs: [usize; 32],
    pub fregs: [usize; 32],
    pub satp: usize,
    pub trap_stack: *mut u8,
    pub hartid: usize,
}


impl TrapFrame{
    pub const fn new() -> Self{
        TrapFrame{
            regs: [0; 32],
            fregs: [0; 32],
            satp: 0,
            trap_stack: null_mut(),
            hartid: 0
        }
    }
}


#[repr(usize)]
pub enum SATP_mode{
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
    Sv64 = 11
}

pub fn satp_read() -> u64{
    let satp_val: u64;
    unsafe{
        asm!("csrr {0}, satp", out(reg) satp_val);
    }

    satp_val
}

pub fn satp_write(satp_mode: SATP_mode, asid_val: usize, root_addr: usize ) {
    let new_satp_val:usize = ((satp_mode as usize) << 60) |
                            (asid_val << 44) |
                            (root_addr >> 12);
    unsafe{
        asm!("csrw satp, {0}", in(reg) new_satp_val);
    }
}

pub fn mepc_read() -> usize{
    let mepc_val: usize;
    unsafe{
        asm!("csrr {0}, mepc", out(reg) mepc_val);
    }

    mepc_val
}

pub fn mepc_write(mepc_new_val: usize) {
    unsafe{
        asm!("csrw mepc, {0}", in(reg) mepc_new_val);
    }
}

pub fn mstatus_read() -> usize{
    let mstatus_val: usize;
    unsafe{
        asm!("csrr {0}, mstatus", out(reg) mstatus_val);
    }

    mstatus_val
}

pub fn mstatus_write(mstatus_new_val: usize) {
    unsafe{
        asm!("csrw mstatus, {0}", in(reg) mstatus_new_val);
    }
}

/*
 * mtvec is actually easier to handle in asm instead of rust
 */
pub fn mtvec_read() -> usize{
    let mtvec_val: usize;
    unsafe{
        asm!("csrr {0}, mtvec", out(reg) mtvec_val);
    }

    return mtvec_val;
}

pub fn mtvec_write(mtvec_new_val: usize) {
    unsafe{
        asm!("csrw mtvec, {0}", in(reg) mtvec_new_val);
    }
}

pub fn mie_read() -> usize{
    let mie_val: usize;
    unsafe{
        asm!("csrr {0}, mie", out(reg) mie_val);
    }

    mie_val
}

pub fn mie_write(mie_new_val: usize) {
    unsafe{
        asm!("csrw mie, {0}", in(reg) mie_new_val);
    }
}


pub fn sie_read() -> usize{
    let sie_val: usize;
    unsafe{
        asm!("csrr {0}, sie", out(reg) sie_val);
    }

    sie_val
}

pub fn sie_write(sie_new_val: usize) {
    unsafe{
        asm!("csrw sie, {0}", in(reg) sie_new_val);
    }
}

pub fn mscratch_read() -> usize{
    let mscratch_val: usize;
    unsafe{
        asm!("csrr {0}, mscratch", out(reg) mscratch_val);
    }

    mscratch_val
}

pub fn mscratch_write(mscratch_new_val: usize) {
    unsafe{
        asm!("csrw mscratch, {0}", in(reg) mscratch_new_val);
    }
}

pub fn sscratch_read() -> usize{
    let sscratch_val: usize;
    unsafe{
        asm!("csrr {0}, sscratch", out(reg) sscratch_val);
    }

    sscratch_val
}

pub fn sscratch_write(sscratch_new_val: usize) {
    unsafe{
        asm!("csrw sscratch, {0}", in(reg) sscratch_new_val);
    }
}

pub fn mhartid_read() -> usize{
    let mhartid_val: usize;
    unsafe{
        asm!("csrr {0}, mhartid", out(reg) mhartid_val);
    }

    mhartid_val
}

pub fn sfence_vma(){
    unsafe{
        asm!("sfence.vma");
    }
}

#[no_mangle]
pub extern "C"
fn which_cpu() -> usize{
    let sp_val: usize;
    let stack_base = addr_of!(_stack_start) as usize;
    unsafe{
        asm!("move {0}, sp", out(reg) sp_val);
    }
    
    ((stack_base - sp_val) / 0x10000) as usize

}

pub fn S_cli() -> usize {
    let mut prev_sie: usize;
    unsafe{
        sstatus::clear_sie();
        prev_sie = sie_read();
        sie_write(0);
    }
    prev_sie
}

pub fn S_sti(prev_sie: usize) {
    unsafe{
        sie_write(prev_sie);
        sstatus::set_sie();
    }
}

pub fn M_cli() -> usize {
    let mut prev_mie: usize;
    unsafe{
        mstatus::clear_mie();
        prev_mie = mie_read();
        mie_write(0);
    }
    prev_mie
}

pub fn M_sti(prev_mie: usize) {
    unsafe{
        mie_write(prev_mie);
        mstatus::set_mie();
    }
}
