//! Older manual task and semaphore probes retained outside the runtime sources.

use crate::cpu::{busy_delay, which_cpu};
use crate::ecall::{trapping, S2Mop};
use crate::kthread::{get_ktpid_lifeid, INVAL_KTHREADS_PID};
use crate::{sem_uart, Sprintln, S_UART};

pub extern "C" fn hello_task() {
    let cpuid = which_cpu();
    let (pid, lifeid) = get_ktpid_lifeid(cpuid).unwrap_or((INVAL_KTHREADS_PID, 0));
    assert_ne!(pid, INVAL_KTHREADS_PID);
    assert_ne!(lifeid, 0);

    loop {
        busy_delay(1);
        Sprintln!("Hello from task pid={} on hart={}", pid, cpuid);
        trapping(S2Mop::YIELD, None);
    }
}

pub extern "C" fn semaphore_waiter() {
    let cpuid = which_cpu();
    let (pid, lifeid) = get_ktpid_lifeid(cpuid).unwrap_or((INVAL_KTHREADS_PID, 0));
    assert_ne!(pid, INVAL_KTHREADS_PID);
    assert_ne!(lifeid, 0);

    loop {
        Sprintln!("semaphore blocked task pid={}", pid);
        unsafe {
            sem_uart.wait();
        }
        Sprintln!("semaphore unblocked task pid={}", pid);
        trapping(S2Mop::YIELD, None);
    }
}
