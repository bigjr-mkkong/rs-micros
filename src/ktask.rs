use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, satp_read, satp_write,
    sscratch_write, which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall::{trapping, S2Mop};
use crate::{M_UART, S_UART};

const fn get_magic() -> usize {
    2233
}

/*
 * Output0:
 * xtval: 0x8d9
 * sepc: 0x800184ba
 */
#[no_mangle]
pub extern "C" fn KHello_task0() {
    loop {
        // busy_delay(1);
        let k = get_magic();
        Sprintln!("Hello from KHello_task0() w/ magic {}", k);
        // trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
    }
}

#[no_mangle]
pub extern "C" fn KHello_task1() {
    loop {
        // busy_delay(1);
        Sprintln!("Hello from KHello_task1()");
        trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
    }
}
