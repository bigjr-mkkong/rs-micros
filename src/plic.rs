/*
 * We are using is SiFive PLIC, here is the spec:
 *https://sifive.cdn.prismic.io/sifive%2F834354f0-08e6-423c-bf1f-0cb58ef14061_fu540-c000-v1.0.pdf#%5B%7B%22num%22%3A164%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C0%2C630%2C0%5D
 */

use core::mem::variant_count;

use crate::SYS_UART;
use crate::{KError, KErrorType};
use crate::new_kerror;
use crate::cpu::{which_cpu, MAX_HARTS};
use crate::lock::spin_mutex;
use crate::lock::{M_lock, S_lock};

pub const PLIC_BASE: usize = 0x0c00_0000;


pub enum extint_map{
    UART0_SENDRECV,
    UART0_LINESTAT,
    TIMER,
    GPIO,
    VIRTIO_NET,
    VIRTIO_BLK,
}

impl extint_map{
    fn id(&self) -> u32{
        let val:u32 = match self{
            extint_map::UART0_SENDRECV =>   0,
            extint_map::UART0_LINESTAT =>   1,
            extint_map::TIMER =>            2,
            extint_map::GPIO =>             3,
            extint_map::VIRTIO_NET =>       4,
            extint_map::VIRTIO_BLK =>       5,
        };

        val
    }
}

pub enum plic_ctx{
    CORE0_M,
    CORE1_M,
    CORE2_M,
    CORE3_M,
    CORE1_S,
    CORE2_S,
    CORE3_S,
}

impl plic_ctx{
    pub fn index(&self) -> usize{
        match self{
            plic_ctx::CORE0_M => 0,
            plic_ctx::CORE1_M => 1,
            plic_ctx::CORE2_M => 3,
            plic_ctx::CORE3_M => 5,
            plic_ctx::CORE1_S => 2,
            plic_ctx::CORE2_S => 4,
            plic_ctx::CORE3_S => 6,
        }
    }

    pub const fn max_ctx() -> usize{
        variant_count::<plic_ctx>()
    }
}


pub struct plic_controller{
    pub base: usize,
    pub prio_base: usize,
    pub pend_base: usize,
    pub enable_base: usize,
    pub thres_base: usize,
    prio: [spin_mutex<u32, M_lock>; 54],
}

impl plic_controller{
    pub const fn new(new_base: usize) -> Self{
        plic_controller{
            base: new_base,
            prio_base: new_base,
            pend_base: new_base + 0x1000,
            enable_base: new_base + 0x2000,
            thres_base: new_base + 0x20_0000,
            prio: [const{spin_mutex::new(0)}; 54],
        }
    }

    pub fn set_prio(&mut self, src:extint_map, new_prio: u32) -> Result<(), KError>{
        if new_prio > 7{
            return Err(new_kerror!(KErrorType::EINVAL));
        }

        let usz_src = src.id() as usize;
        *self.prio[usz_src].lock() = new_prio;

        unsafe{
            let base_pt = self.prio_base as *mut u32;
            base_pt.add(usz_src).write(new_prio as u32)
        }

        Ok(())
    }

    pub fn get_prio(self, src:extint_map) -> Result<u32, KError>{
        let usz_src = src.id() as usize;
        let reg_val = *self.prio[usz_src].lock();

        let mut mmio_val: u32;
        unsafe{
            let base_pt = self.prio_base as *mut u32;
            mmio_val = base_pt.add(usz_src).read();
        }

        if mmio_val != reg_val{
            return Err(new_kerror!(KErrorType::EFAULT));
        }else{
            return Ok(reg_val);
        }
    }

    pub fn get_pending(&self, src:extint_map) -> Result<bool, KError>{
        let usz_src = src.id() as usize;
        unsafe{
            let pend_pt = self.pend_base as *mut u32;
            
            let pend_reg: u32 = pend_pt.add(usz_src / 32).read();
            let mask: u32 = (1 << (usz_src % 32));

            if pend_reg & mask == 0{
                return Ok(false);
            }else{
                return Ok(true);
            }
        }
    }

    
    pub fn enable(&self, ctx: plic_ctx, src: extint_map) -> Result<(), KError>{
        let usz_ctx = ctx.index() as usize;
        let usz_src:usize = src.id() as usize / 32;
        let mask:u32 = (1 << (src.id() % 32));
        unsafe{
            let enable_pt = (self.enable_base + (usz_ctx * 32 + usz_src)) as *mut u32;
            let mut enable_reg:u32 = enable_pt.read();
            enable_reg = enable_reg | mask;
            enable_pt.write(enable_reg);
        }

        Ok(())
    }

    pub fn set_thres(&self, ctx: plic_ctx, new_thres: u32) -> Result<(), KError>{
        if new_thres > 7{
            return Err(new_kerror!(KErrorType::EFAULT));
        }
        let usz_ctx = ctx.index() as usize;
        let thres_base = self.thres_base as *mut u32;

        unsafe{
            thres_base.add(usz_ctx * 0x1000).write(new_thres);
        }

        Ok(())
    }

    pub fn claim(&self, ctx: plic_ctx) -> Result<u32, KError>{
        let usz_ctx = ctx.index() as usize;
        let claim_base = self.thres_base as *mut u32;
        let mut claimed_int: u32;

        unsafe{
            claimed_int = claim_base.add(0x1000 * usz_ctx + 1).read();
        }

        Ok(claimed_int)
    }

    pub fn complete(&self, ctx: plic_ctx, src: u32) -> Result<(), KError>{
        let usz_ctx = ctx.index() as usize;
        let claim_base = self.thres_base as *mut u32;

        unsafe{
            claim_base.add(0x1000 * usz_ctx + 1).write(src);
        }

        Ok(())
    }
}


