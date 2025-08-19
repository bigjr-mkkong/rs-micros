use crate::alloc::boxed::Box;
use crate::alloc::vec::Vec;
use crate::cpu::{get_cpu_mode, Mode};
use crate::ecall::{trapping, S2Mop};
use crate::error::{KError, KErrorType};
use crate::kthread::INVAL_KTHREADS_PID;
use crate::kthread::{get_ktpid_lifeid, task_state};
use crate::lock::spin_mutex;
use crate::lock::{Critical_Area, M_lock, S_lock};
use crate::new_kerror;
use crate::task_struct;
use crate::which_cpu;
use crate::Mprintln;
use crate::KTHREAD_POOL;
use crate::{M_UART, S_UART};

// (pid, lifeid)
pub struct kt_semaphore {
    cnt: spin_mutex<i32, S_lock>,
    wait_q: spin_mutex<Vec<(usize, usize)>, Critical_Area>,
}

impl kt_semaphore {
    pub const fn new(new_cnt: i32) -> Self {
        assert!(new_cnt >= 0, "Semaphore must be non-negative!");
        Self {
            cnt: spin_mutex::new(new_cnt),
            wait_q: spin_mutex::new(Vec::new()),
        }
    }

    pub fn wait(&mut self) {
        let cpuid = which_cpu();
        let (pid, lifeid) = get_ktpid_lifeid(cpuid).unwrap_or((INVAL_KTHREADS_PID, 0));
        assert_ne!(pid, INVAL_KTHREADS_PID);
        assert_ne!(lifeid, 0);

        let mut cnt = self.cnt.lock();
        *cnt -= 1;
        if *cnt < 0 {
            let mut wait_q = self.wait_q.lock();
            wait_q.push((pid, lifeid));
            drop(cnt);
            drop(wait_q);
            trapping(S2Mop::BLOCK, Some(&[pid, lifeid, 0, 0, 0]));
        } else {
            drop(cnt);
        }
    }

    pub fn signal(&mut self, hart: Option<usize>) {
        let cpuid = hart.unwrap_or(INVAL_KTHREADS_PID);
        assert_ne!(cpuid, INVAL_KTHREADS_PID);
        let mut cnt = self.cnt.lock();
        *cnt += 1;

        if *cnt <= 0 {
            drop(cnt);

            let mut wait_q = self.wait_q.lock();
            if let Some((wake_pid, lifeid)) = wait_q.pop() {
                let current_mode = get_cpu_mode(cpuid);
                if matches!(current_mode, Mode::Machine | Mode::Machine_IRH) {
                    unsafe {
                        KTHREAD_POOL.set_state_by_pid(wake_pid, lifeid, task_state::Ready);
                    }
                } else {
                    drop(wait_q);
                    trapping(S2Mop::UNBLOCK, Some(&[wake_pid, lifeid, 0, 0, 0]));
                }
            }
        } else {
            drop(cnt);
        }
    }
}
