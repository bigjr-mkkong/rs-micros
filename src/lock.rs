use spin::{Mutex, RwLock};
use crate::cpu::{cli, sti};
use core::ops::{Deref, DerefMut};
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
            dat: self.inner_lock.lock()
        }
    }


}

pub struct irq_mutex_guard<'a, T> {
    pub dat: spin::MutexGuard<'a, T>,
}

impl<T> Drop for irq_mutex_guard<'_, T>{
    fn drop(&mut self) {
        sti()
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
        cli();

        let guard = self.inner_lock.write();

        irq_rwlock_writeguard{
            dat: Some(guard)
        }
    }


    pub fn read(&self) -> irq_rwlock_readguard<'_, T> {
        cli();

        let guard = self.inner_lock.read();

        irq_rwlock_readguard{
            dat: guard
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
