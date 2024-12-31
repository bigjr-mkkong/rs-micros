use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, satp_write, sscratch_write,
    which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall::{trapping, S2Mop};
use crate::{M_UART, S_UART};

#[no_mangle]
pub extern "C" fn KHello_task0() {
    loop {
        // busy_delay(1);
        Sprintln!("Hello from KHello_task0() on CPU#{}", which_cpu());
        trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
    }
}

#[no_mangle]
pub extern "C" fn KHello_task1() {
    loop {
        // busy_delay(1);
        Sprintln!("Hello from KHello_task1() on CPU#{}", which_cpu());
        trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
    }
}
