use core::ptr::{null_mut, addr_of};
use core::arch::asm;
use core::ops::{Deref, DerefMut};
use riscv::register::{sie, mie, mstatus};
use spin::{Mutex, RwLock};
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


pub fn menvcfg_read() -> u64{
    let menvcfg_val: u64;
    unsafe{
        asm!("csrr {0}, 0x30a", out(reg) menvcfg_val);
    }

    menvcfg_val
}

pub fn menvcfg_write(menvcfg_new_val: u64) {
    unsafe{
        asm!("csrw 0x30a, {0}", in(reg) menvcfg_new_val);
    }
}

pub fn mcounteren_read() -> u64{
    let mcounteren_val: u64;
    unsafe{
        asm!("csrr {0}, mcounteren", out(reg) mcounteren_val);
    }

    mcounteren_val
}

pub fn mcounteren_write(mcounteren_new_val: u64) {
    unsafe{
        asm!("csrw mcounteren, {0}", in(reg) mcounteren_new_val);
    }
}


pub fn stimecmp_write(stimecmp_new_val: u64) {
    unsafe{
        asm!("csrw stimecmp, {0}", in(reg) stimecmp_new_val);
    }
}


pub fn time_read() -> u64{
    let time_val: u64;
    unsafe{
        asm!("csrr {0}, time", out(reg) time_val);
    }

    time_val
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

pub fn cli(){
    unsafe{
        asm!("csrc  sstatus, {}", in(reg) 1 << 1);
        // sie::clear_stimer();
    }

}

pub fn sti() {
    unsafe{
        asm!("csrs  sstatus, {}", in(reg) 1 << 1);
        // sie::set_stimer();
    }
}

pub struct irq_mutex<T>{
    inner_lock: Mutex<T>
}

impl<T> irq_mutex<T> {
    pub const fn new(dat: T) -> Self{
        Self{
            inner_lock: Mutex::new(dat),
        }
    }

    pub fn lock(&self) -> irq_mutex_guard<'_, T> {
        cli();

        irq_mutex_guard{
            dat: self.inner_lock.lock(),
        }
    }


}

pub struct irq_mutex_guard<'a, T> {
    pub dat: spin::MutexGuard<'a, T>,
}

impl<T> Drop for irq_mutex_guard<'_, T>{
    fn drop(&mut self) {
        sti();
    }
}

impl <'a, T> Deref for irq_mutex_guard<'a, T>{
    type Target = T;

    fn deref(&self) -> &Self::Target{
        &self.dat
    }
}


impl <'a, T> DerefMut for irq_mutex_guard<'a, T>{
    fn deref_mut(&mut self) -> &mut Self::Target{
        &mut self.dat
    }
}

pub struct irq_rwlock<T>{
    inner_lock: RwLock<T>
}

pub struct irq_rwlock_writeguard<'a, T> {
    pub dat: Option<spin::RwLockWriteGuard<'a, T>>,
}

impl<T> irq_rwlock<T> {
    pub const fn new(dat: T) -> Self{
        Self{
            inner_lock: RwLock::new(dat),
        }
    }

    pub fn write(&self) -> irq_rwlock_writeguard<'_, T> {
        let prev_sie = cli();

        let guard = self.inner_lock.write();

        irq_rwlock_writeguard{
            dat: Some(guard),
        }
    }


    pub fn read(&self) -> irq_rwlock_readguard<'_, T> {
        cli();

        let guard = self.inner_lock.read();

        irq_rwlock_readguard{
            dat: guard,
        }
    }
}


impl<T> Drop for irq_rwlock_writeguard<'_, T>{
    fn drop(&mut self) {
        sti()
    }
}

impl <'a, T> Deref for irq_rwlock_writeguard<'a, T>{
    type Target = T;

    fn deref(&self) -> &Self::Target{
        &self.dat.as_ref().unwrap()
    }
}

impl <'a, T> DerefMut for irq_rwlock_writeguard<'a, T>{
    fn deref_mut(&mut self) -> &mut Self::Target{
        self.dat.as_mut().unwrap()
    }
}


pub struct irq_rwlock_readguard<'a, T> {
    pub dat: spin::RwLockReadGuard<'a, T>,
}

impl<T> Drop for irq_rwlock_readguard<'_, T> {
    fn drop(&mut self) {
        sti()
    }
}

impl <'a, T> Deref for irq_rwlock_readguard<'a, T>{
    type Target = T;

    fn deref(&self) -> &Self::Target{
        &self.dat
    }
}



pub fn timer_init(){
    unsafe{
        mie::set_stimer();
    }
    
    menvcfg_write(menvcfg_read() | ((1 as u64) << 63));

    mcounteren_write(mcounteren_read() | 2);
    
    boost_timer(1);
}

pub fn boost_timer(sec: usize){
    stimecmp_write(time_read() + (sec as u64) * 1_000);
}
