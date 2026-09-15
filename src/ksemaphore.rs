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
            // Publish the blocked state while holding cnt. A signaler cannot
            // pass us until both the state and wait-queue entry are visible.
            // Marking the task blocked later in the BLOCK ecall allowed a
            // signal to wake it first and then have BLOCK overwrite Ready.
            unsafe {
                KTHREAD_POOL
                    .set_current_state(cpuid, task_state::Block)
                    .unwrap_or_else(|_| panic!("failed to block semaphore waiter"));
            }

            let mut wait_q = self.wait_q.lock();
            wait_q.push((pid, lifeid));
            drop(cnt);
            drop(wait_q);
            trapping(S2Mop::BLOCK, None);
        } else {
            drop(cnt);
        }
    }

    fn take_waiter(&mut self) -> Option<(usize, usize)> {
        let mut cnt = self.cnt.lock();
        *cnt += 1;

        if *cnt <= 0 {
            drop(cnt);

            let mut wait_q = self.wait_q.lock();
            wait_q.pop()
        } else {
            drop(cnt);
            None
        }
    }

    pub fn signal(&mut self, hart: Option<usize>) {
        let cpuid = hart.unwrap_or(INVAL_KTHREADS_PID);
        assert_ne!(cpuid, INVAL_KTHREADS_PID);

        if let Some((wake_pid, lifeid)) = self.take_waiter() {
            let current_mode = get_cpu_mode(cpuid);
            if matches!(current_mode, Mode::Machine | Mode::Machine_IRH) {
                unsafe {
                    KTHREAD_POOL
                        .set_state_by_pid(wake_pid, lifeid, task_state::Ready)
                        .unwrap_or_else(|_| panic!("failed to wake semaphore waiter"));
                }
            } else {
                trapping(S2Mop::UNBLOCK, Some(&[wake_pid, lifeid, 0, 0, 0]));
            }
        }
    }

    /// Wake a waiter while already executing inside a trap handler.
    ///
    /// Calling `signal()` from `s_trap` would issue a nested S-mode ecall and
    /// overwrite the interrupted task's trap frame.  The handler already has
    /// kernel control, so it can publish the Ready state directly.
    pub fn signal_from_trap(&mut self) {
        if let Some((wake_pid, lifeid)) = self.take_waiter() {
            unsafe {
                KTHREAD_POOL
                    .set_state_by_pid(wake_pid, lifeid, task_state::Ready)
                    .unwrap_or_else(|_| panic!("failed to wake semaphore waiter from trap"));
            }
        }
    }
}
