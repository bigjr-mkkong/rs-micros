use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, satp_read, satp_write,
    sscratch_write, which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall::{trapping, S2Mop};
use crate::{M_UART, S_UART};
use alloc::vec::Vec;

#[no_mangle]
pub extern "C" fn KHello_task0() {
    loop{
        // busy_delay(1);
        Sprintln!("Hello from KHello_task0() from CPU#{}", which_cpu());
        trapping(S2Mop::YIELD, None);
    }
}

#[no_mangle]
pub extern "C" fn KHello_task1() {
    loop{
        // busy_delay(1);
        Sprintln!("Hello from KHello_task1() from CPU#{}", which_cpu());
        trapping(S2Mop::YIELD, None);
    }
}

#[no_mangle]
pub extern "C" fn ktask_uart() {
    Sprintln!("CPU#{} trapped at uart, but I will not tell you wats goinon", which_cpu());
    trapping(S2Mop::EXIT, None);
}

#[no_mangle]
pub extern "C" fn ktask_fallback() {
    Sprintln!("CPU#{} at ktask fallbacker, trying to yield...", which_cpu());
    trapping(S2Mop::YIELD, None);
}
