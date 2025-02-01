use crate::alloc::boxed::Box;
use crate::alloc::vec::Vec;
use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::lock::spin_mutex;
use crate::{task_struct};
use crate::lock::{Critical_Area, M_lock, S_lock};

struct semaphore{
    cnt: spin_mutex<i32, S_lock>,
    wait_q: Vec<usize>
}

impl semaphore {
    pub const fn new(new_cnt: i32) -> Self{
        assert!(new_cnt >= 0, "Semaphore must be non-negative!");
        Self{
            cnt: spin_mutex::new(new_cnt),
            wait_q: Vec::new()
        }
    }

    pub fn wait(&mut self) {

    }

    pub fn signal(&mut self) {

    }
}
