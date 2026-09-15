//! In-image QEMU stress test for the multicore scheduler, semaphore, and UART lock.

use crate::cpu::{which_cpu, MAX_HARTS};
use crate::ecall::{trapping, S2Mop};
use crate::error::KError;
use crate::ksemaphore::kt_semaphore;
use crate::kthread::{get_ktpid_lifeid, task_flag, INVAL_KTHREADS_PID};
use crate::{Sprintln, KTHREAD_POOL, S_UART};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const WORKERS_PER_HART: usize = 2;
const ITERATIONS: usize = 16;
const WORKER_TOTAL: usize = MAX_HARTS * WORKERS_PER_HART;
const WAKEUPS_PER_HART: usize = 16;
const SEMAPHORE_SIGNALS: usize = MAX_HARTS * WAKEUPS_PER_HART;

static WORKERS_DONE: AtomicUsize = AtomicUsize::new(0);
static WAITERS_READY: AtomicUsize = AtomicUsize::new(0);
static WAITERS_DONE: AtomicUsize = AtomicUsize::new(0);
static SEMAPHORE_WAKES: AtomicUsize = AtomicUsize::new(0);
static PHASES_DONE: AtomicUsize = AtomicUsize::new(0);
static FINISHED: AtomicBool = AtomicBool::new(false);
static mut SEMAPHORE: kt_semaphore = kt_semaphore::new(0);

fn finish_phase(phase: usize) {
    let phases = PHASES_DONE.fetch_or(phase, Ordering::AcqRel) | phase;
    if phases == 0b11 && !FINISHED.swap(true, Ordering::AcqRel) {
        // MTIP must be serviced independently on every hart while S-mode is
        // busy scheduling and printing. Give the slowest hart a bounded
        // window to receive its first tick before declaring success.
        for _ in 0..10_000_000 {
            if (0..MAX_HARTS).all(|hart| crate::trap::machine_timer_ticks(hart) > 0) {
                break;
            }
            core::hint::spin_loop();
        }
        let machine_timer_ticks: usize = (0..MAX_HARTS).map(crate::trap::machine_timer_ticks).sum();
        assert!((0..MAX_HARTS).all(|hart| crate::trap::machine_timer_ticks(hart) > 0));

        Sprintln!(
            "[MULTICORE-TEST] PASS workers={} semaphore_waiters={} semaphore_wakes={} machine_timer_ticks={}",
            WORKERS_DONE.load(Ordering::Acquire),
            WAITERS_DONE.load(Ordering::Acquire),
            SEMAPHORE_WAKES.load(Ordering::Acquire),
            machine_timer_ticks
        );

        // Let the final UART line drain before terminating QEMU through its
        // SiFive test device.
        for _ in 0..100_000 {
            core::hint::spin_loop();
        }

        unsafe {
            (0x0010_0000 as *mut u32).write_volatile(0x5555);
        }
    }
}

extern "C" fn scheduler_worker() {
    let hart = which_cpu();
    let (pid, lifeid) = get_ktpid_lifeid(hart).unwrap_or((INVAL_KTHREADS_PID, 0));
    assert_ne!(pid, INVAL_KTHREADS_PID);
    assert_ne!(lifeid, 0);

    for iteration in 0..ITERATIONS {
        Sprintln!(
            "[KTASK-STRESS] hart={} pid={} iter={}",
            hart,
            pid,
            iteration
        );
        trapping(S2Mop::YIELD, None);
    }

    if WORKERS_DONE.fetch_add(1, Ordering::AcqRel) + 1 == WORKER_TOTAL {
        finish_phase(0b01);
    }

    loop {
        trapping(S2Mop::YIELD, None);
    }
}

extern "C" fn semaphore_waiter() {
    let hart = which_cpu();
    let (pid, lifeid) = get_ktpid_lifeid(hart).unwrap_or((INVAL_KTHREADS_PID, 0));
    assert_ne!(pid, INVAL_KTHREADS_PID);
    assert_ne!(lifeid, 0);
    WAITERS_READY.fetch_add(1, Ordering::Release);

    for _ in 0..WAKEUPS_PER_HART {
        unsafe {
            SEMAPHORE.wait();
        }

        let wake = SEMAPHORE_WAKES.fetch_add(1, Ordering::AcqRel) + 1;
        Sprintln!("[KSEM-STRESS] hart={} pid={} wake={}", hart, pid, wake);
    }

    if WAITERS_DONE.fetch_add(1, Ordering::AcqRel) + 1 == MAX_HARTS {
        finish_phase(0b10);
    }

    loop {
        trapping(S2Mop::YIELD, None);
    }
}

extern "C" fn semaphore_signaler() {
    let hart = which_cpu();
    while WAITERS_READY.load(Ordering::Acquire) < MAX_HARTS {
        trapping(S2Mop::YIELD, None);
    }

    for _ in 0..SEMAPHORE_SIGNALS {
        unsafe {
            SEMAPHORE.signal(Some(hart));
        }
        trapping(S2Mop::YIELD, None);
    }

    loop {
        trapping(S2Mop::YIELD, None);
    }
}

pub(crate) unsafe fn start(cpuid: usize, spawn_signaler: bool) -> Result<(), KError> {
    for _ in 0..WORKERS_PER_HART {
        KTHREAD_POOL.spawn(scheduler_worker as usize, task_flag::NORMAL, cpuid)?;
    }
    KTHREAD_POOL.spawn(semaphore_waiter as usize, task_flag::NORMAL, cpuid)?;
    if spawn_signaler {
        KTHREAD_POOL.spawn(semaphore_signaler as usize, task_flag::NORMAL, cpuid)?;
    }
    KTHREAD_POOL.join_all_ktask(cpuid)?;
    Ok(())
}
