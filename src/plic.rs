use core::mem::variant_count;

use crate::cpu::{get_cpu_mode, which_cpu, Mode, MAX_HARTS};
use crate::lock::spin_mutex;
use crate::lock::{M_lock, S_lock};
use crate::new_kerror;
use crate::{KError, KErrorType};
use crate::{M_UART, S_UART};
use get_set_macro::get_set;
use spin::Mutex;

pub const PLIC_BASE: usize = 0x0c00_0000;
pub const MAX_INTCNT: usize = 53;

#[derive(Clone, Copy)]
pub enum extint_name {
    UNDEF,
    UART0,
}

#[get_set(get_copy(inline_always, vis = "pub"), set(inline_always, vis = "pub"))]
#[derive(Clone, Copy)]
pub struct extint_src {
    #[gsflags(get(inline_always, vis = "pub"))]
    name: extint_name,
    src_id: usize,
    prio: usize,
}

impl extint_src {
    pub const fn new() -> Self {
        extint_src {
            name: extint_name::UNDEF,
            src_id: 0,
            prio: 0,
        }
    }
}

pub enum plic_ctx {
    CORE0_M,
    CORE1_M,
    CORE2_M,
    CORE3_M,
    CORE0_S,
    CORE1_S,
    CORE2_S,
    CORE3_S,
}

impl plic_ctx {
    pub fn index(&self) -> usize {
        match self {
            plic_ctx::CORE0_M => 0,
            plic_ctx::CORE1_M => 2,
            plic_ctx::CORE2_M => 4,
            plic_ctx::CORE3_M => 6,
            plic_ctx::CORE0_S => 1,
            plic_ctx::CORE1_S => 3,
            plic_ctx::CORE2_S => 5,
            plic_ctx::CORE3_S => 7,
        }
    }

    pub const fn max_ctx() -> usize {
        variant_count::<plic_ctx>()
    }
}

pub struct plic_controller {
    pub base: usize,
    pub prio_base: usize,
    pub pend_base: usize,
    pub enable_base: usize,
    pub thres_base: usize,
    prio: [spin_mutex<u32, M_lock>; 54],
}

impl plic_controller {
    pub const fn new(new_base: usize) -> Self {
        plic_controller {
            base: new_base,
            prio_base: new_base,
            pend_base: new_base + 0x1000,
            enable_base: new_base + 0x2000,
            thres_base: new_base + 0x20_0000,
            prio: [const { spin_mutex::new(0) }; 54],
        }
    }

    pub fn set_prio(&mut self, src: &extint_src, new_prio: u32) -> Result<(), KError> {
        if new_prio > 7 {
            return Err(new_kerror!(KErrorType::EINVAL));
        }

        let usz_src = src.get_src_id();
        *self.prio[usz_src].lock() = new_prio;

        unsafe {
            let base_pt = self.prio_base as *mut u32;
            base_pt.add(usz_src).write(new_prio as u32)
        }

        Ok(())
    }

    pub fn get_prio(self, src: &extint_src) -> Result<u32, KError> {
        let usz_src = src.get_src_id();
        let reg_val = *self.prio[usz_src].lock();

        let mut mmio_val: u32;
        unsafe {
            let base_pt = self.prio_base as *mut u32;
            mmio_val = base_pt.add(usz_src).read();
        }

        if mmio_val != reg_val {
            return Err(new_kerror!(KErrorType::EFAULT));
        } else {
            return Ok(reg_val);
        }
    }

    pub fn get_pending(&self, src: &extint_src) -> Result<bool, KError> {
        let usz_src = src.get_src_id();
        unsafe {
            let pend_pt = self.pend_base as *mut u32;

            let pend_reg: u32 = pend_pt.add(usz_src / 32).read();
            let mask: u32 = (1 << (usz_src % 32));

            if pend_reg & mask == 0 {
                return Ok(false);
            } else {
                return Ok(true);
            }
        }
    }

    pub fn enable(&self, ctx: plic_ctx, src: &extint_src) -> Result<(), KError> {
        let usz_ctx = ctx.index() as usize;
        let usz_src: usize = src.get_src_id() / 32;
        let mask: u32 = (1 << (src.get_src_id() % 32));
        unsafe {
            let enable_pt = (self.enable_base + (usz_ctx * 0x80 + usz_src)) as *mut u32;
            let mut enable_reg: u32 = enable_pt.read();
            enable_reg = enable_reg | mask;
            enable_pt.write(enable_reg);
        }

        Ok(())
    }

    pub fn disable(&self, ctx: plic_ctx, src: &extint_src) -> Result<(), KError> {
        let usz_ctx = ctx.index() as usize;
        let usz_src: usize = src.get_src_id() / 32;
        let mask: u32 = !(1 << (src.get_src_id() % 32));
        unsafe {
            let disable_pt = (self.enable_base + (usz_ctx * 0x80 + usz_src)) as *mut u32;
            let mut disable_reg: u32 = disable_pt.read();
            disable_reg = disable_reg | mask;
            disable_pt.write(disable_reg);
        }

        Ok(())
    }
    pub fn set_thres(&self, ctx: plic_ctx, new_thres: u32) -> Result<(), KError> {
        if new_thres > 7 {
            return Err(new_kerror!(KErrorType::EFAULT));
        }
        let usz_ctx = ctx.index() as usize;
        let thres_base = self.thres_base as *mut u32;

        unsafe {
            thres_base.add(usz_ctx * 0x400).write_volatile(new_thres);
        }

        Ok(())
    }

    pub fn claim(&self, ctx: &plic_ctx) -> Result<u32, KError> {
        let usz_ctx = ctx.index() as usize;
        let claim_base = self.thres_base as *mut u32;
        let mut claimed_int: u32;

        unsafe {
            let claim_addr = claim_base.add(0x400 * usz_ctx + 1);
            claimed_int = claim_addr.read_volatile();
        }

        Ok(claimed_int)
    }

    pub fn complete(&self, ctx: &plic_ctx, src: u32) -> Result<(), KError> {
        let usz_ctx = ctx.index() as usize;
        let claim_base = self.thres_base as *mut u32;

        unsafe {
            claim_base.add(0x400 * usz_ctx + 1).write(src);
        }

        Ok(())
    }
}

pub fn id2plic_ctx(hartid: usize) -> plic_ctx {
    let current_mode = get_cpu_mode(hartid);
    if matches!(current_mode, Mode::Machine | Mode::Machine_IRH) {
        match hartid {
            0 => plic_ctx::CORE0_M,
            1 => plic_ctx::CORE1_M,
            2 => plic_ctx::CORE2_M,
            3 => plic_ctx::CORE3_M,
            _ => {
                panic!();
            }
        }
    } else {
        match hartid {
            0 => plic_ctx::CORE0_S,
            1 => plic_ctx::CORE1_S,
            2 => plic_ctx::CORE2_S,
            3 => plic_ctx::CORE3_S,
            _ => {
                panic!();
            }
        }
    }
}
