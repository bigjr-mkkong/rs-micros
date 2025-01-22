use crate::cpu::{get_cpu_mode, which_cpu, M_cli, M_sti, Mode, S_cli, S_sti};
use crate::ecall::{trapping, S2Mop};
use crate::{M_UART, S_UART};
use core::arch::asm;
use core::ops::{Deref, DerefMut};
use crate::SECALL_FRAME;
use spin::{Mutex, RwLock};

pub trait IntControl {
    fn cli() -> usize;
    fn sti(prev_xie: usize);
}

pub struct M_lock;
pub struct S_lock;
pub struct Critical_Area;

impl IntControl for M_lock {
    fn cli() -> usize {
        // M_cli()
        0
    }

    fn sti(prev_xie: usize) {
        // M_sti(prev_xie)
    }
}

impl IntControl for S_lock {
    fn cli() -> usize {
        S_cli()
    }

    fn sti(prev_xie: usize) {
        S_sti(prev_xie)
    }
}

// impl IntControl for Critical_Area{
//     fn cli() -> usize{
//         let cpuid = which_cpu();
//         assert_eq!(cpuid, 0);
//         trapping(S2Mop::CLI, None);
//         unsafe{
//             SECALL_FRAME[cpuid].get_ret()
//         }
//     }

//     fn sti(prev_xie: usize){
//         trapping(S2Mop::STI, Some(&[prev_xie, 0, 0, 0, 0]));
//     }
// }

pub struct spin_mutex<T, MODE: IntControl> {
    inner_lock: Mutex<T>,
    _mode: core::marker::PhantomData<MODE>,
}

impl<T, MODE: IntControl> spin_mutex<T, MODE> {
    pub const fn new(dat: T) -> Self {
        Self {
            inner_lock: Mutex::new(dat),
            _mode: core::marker::PhantomData,
        }
    }

    pub fn lock(&self) -> spin_mutex_guard<'_, T, MODE> {
        let prev_xie = MODE::cli();

        spin_mutex_guard::<T, MODE> {
            dat: self.inner_lock.lock(),
            old_xie: prev_xie,
            _mode: core::marker::PhantomData,
        }
    }
}

pub struct spin_mutex_guard<'a, T, MODE: IntControl> {
    pub dat: spin::MutexGuard<'a, T>,
    old_xie: usize,
    _mode: core::marker::PhantomData<MODE>,
}

impl<T, MODE: IntControl> Drop for spin_mutex_guard<'_, T, MODE> {
    fn drop(&mut self) {
        MODE::sti(self.old_xie);
    }
}

impl<'a, T, MODE: IntControl> Deref for spin_mutex_guard<'a, T, MODE> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.dat
    }
}

impl<'a, T, MODE: IntControl> DerefMut for spin_mutex_guard<'a, T, MODE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.dat
    }
}

// pub struct irq_rwlock<T>{
//     inner_lock: RwLock<T>
// }

// pub struct irq_rwlock_writeguard<'a, T> {
//     pub dat: Option<spin::RwLockWriteGuard<'a, T>>,
// }

// impl<T> irq_rwlock<T> {
//     pub const fn new(dat: T) -> Self{
//         Self{
//             inner_lock: RwLock::new(dat),
//         }
//     }

//     pub fn write(&self) -> irq_rwlock_writeguard<'_, T> {
//         cli();

//         let guard = self.inner_lock.write();

//         irq_rwlock_writeguard{
//             dat: Some(guard)
//         }
//     }

//     pub fn read(&self) -> irq_rwlock_readguard<'_, T> {
//         cli();

//         let guard = self.inner_lock.read();

//         irq_rwlock_readguard{
//             dat: guard
//         }
//     }
// }

// impl<T> Drop for irq_rwlock_writeguard<'_, T>{
//     fn drop(&mut self) {
//         sti()
//     }
// }

// impl <'a, T> Deref for irq_rwlock_writeguard<'a, T>{
//     type Target = T;

//     fn deref(&self) -> &Self::Target{
//         &self.dat.as_ref().unwrap()
//     }
// }

// impl <'a, T> DerefMut for irq_rwlock_writeguard<'a, T>{
//     fn deref_mut(&mut self) -> &mut Self::Target{
//         self.dat.as_mut().unwrap()
//     }
// }

// pub struct irq_rwlock_readguard<'a, T> {
//     pub dat: spin::RwLockReadGuard<'a, T>,
// }

// impl<T> Drop for irq_rwlock_readguard<'_, T> {
//     fn drop(&mut self) {
//         sti()
//     }
// }

// impl <'a, T> Deref for irq_rwlock_readguard<'a, T>{
//     type Target = T;

//     fn deref(&self) -> &Self::Target{
//         &self.dat
//     }
// }
