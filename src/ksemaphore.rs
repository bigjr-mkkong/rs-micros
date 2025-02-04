use crate::alloc::boxed::Box;
use crate::alloc::vec::Vec;
use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::lock::spin_mutex;
use crate::{task_struct};
use crate::lock::{Critical_Area, M_lock, S_lock};
use crate::proc::get_ktpid;
use crate::which_cpu;
use crate::ecall::{trapping, S2Mop};

pub struct kt_semaphore{
    cnt: spin_mutex<i32, S_lock>,
    wait_q: spin_mutex<Vec<usize>, Critical_Area>
}

impl kt_semaphore {
    pub const fn new(new_cnt: i32) -> Self{
        assert!(new_cnt >= 0, "Semaphore must be non-negative!");
        Self{
            cnt: spin_mutex::new(new_cnt),
            wait_q: spin_mutex::new(Vec::new())
        }
    }

    pub fn wait(&mut self) {
        let cpuid = which_cpu();
        let pid = get_ktpid(cpuid).unwrap_or(1000);
        assert_ne!(pid, 1000);

        let mut cnt = self.cnt.lock();
        *cnt -= 1;
        if *cnt < 0 {
            let mut wait_q = self.wait_q.lock();
            wait_q.push(pid);
            drop(cnt);
            drop(wait_q);
            trapping(S2Mop::BLOCK, Some(&[pid, 0, 0, 0, 0]));
        } else {
            drop(cnt);
        }
    }

    pub fn signal(&mut self) {
        let mut cnt = self.cnt.lock();
        *cnt += 1;

        if *cnt <= 0 {
            drop(cnt);

            let mut wait_q = self.wait_q.lock();
            if let Some(wake_pid) = wait_q.pop() {
                drop(wait_q);
                trapping(S2Mop::UNBLOCK, Some(&[wake_pid, 0, 0, 0, 0]));
            }
        } else {
            drop(cnt);
        }
    }

}
