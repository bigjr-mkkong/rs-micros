use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, satp_read, satp_write,
    sscratch_write, which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall::{trapping, S2Mop};
use crate::IRQ_BUFFER;
use crate::{M_UART, S_UART};
use alloc::vec::Vec;
use crate::proc::get_ktpid;
use crate::sem_test;


#[no_mangle]
pub extern "C" fn KHello_task0() {
    let cpuid = which_cpu();
    let pid: usize = get_ktpid(cpuid).unwrap_or(1000);
    loop {
        busy_delay(1);
        
        Sprintln!("Hello from KHello_task0(pid: {}) from CPU#{}", pid, cpuid);
        trapping(S2Mop::YIELD, None);
    }
}

#[no_mangle]
pub extern "C" fn KHello_task1() {
    let cpuid = which_cpu();
    let pid: usize = get_ktpid(cpuid).unwrap_or(1000);
    loop {
        busy_delay(1);
        
        Sprintln!("Hello from KHello_task1(pid: {}) from CPU#{}", pid, cpuid);
        trapping(S2Mop::YIELD, None);
    }
}

#[no_mangle]
pub extern "C" fn ksem_test0() {
    let cpuid = which_cpu();
    let pid = get_ktpid(cpuid).unwrap_or(1000);
    assert_ne!(pid, 1000);
    unsafe{
        loop{
            Sprintln!("sem blocked on task#{}", pid);
            sem_test.wait();
            Sprintln!("sem unblocked");
            trapping(S2Mop::YIELD, None);
        }
    }
    trapping(S2Mop::EXIT, None);
}

#[no_mangle]
pub extern "C" fn ktask_extint() {
    let cpuid = which_cpu();
    loop {
        unsafe {
            if let Ok(is_empty) = IRQ_BUFFER.is_empty(cpuid) {
                if is_empty {
                    trapping(S2Mop::YIELD, None);
                } else {
                    unsafe{
                        sem_test.signal();
                    }
                    let new_req = IRQ_BUFFER.peek_req(cpuid).unwrap_or_default().unwrap();
                    IRQ_BUFFER.dequeue_req(cpuid);

                    let hart = new_req.get_cpuid();
                    let extint_id = new_req.get_extint_id();
                    let data = new_req.get_data();

                    match extint_id {
                        10 => {
                            if let Some(ch) = data {
                                let ch = ch as u8;
                                Mprintln!("Uart extint at CPU#{}: {}", hart, ch as char);
                            } else {
                                Mprintln!("Uart extint at CPU#{}: Failed", hart);
                            }
                        }
                        0 => {
                            //do nothing when 0
                        }
                        _ => {
                            Mprintln!("Unsupported extint: #{} on CPU#{}", extint_id, hart);
                        }
                    }
                }
            }
        }
    }
    trapping(S2Mop::EXIT, None);
}

#[no_mangle]
pub extern "C" fn ktask_fallback() {
    Sprintln!(
        "CPU#{} at ktask fallbacker, trying to yield...",
        which_cpu()
    );
    trapping(S2Mop::YIELD, None);
}


#[no_mangle]
pub extern "C" fn paniker() {
    loop{}
}
