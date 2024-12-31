use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, satp_write, sscratch_write,
    which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall::{trapping, S2Mop};
use crate::{M_UART, S_UART};

#[no_mangle]
pub extern "C" fn KHello_cpu0() {
    loop {
        // busy_delay(1);
        println!("Hello from KHello_cpu0()");
        trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
    }
}

#[no_mangle]
pub extern "C" fn second_task() {
    loop {
        // busy_delay(1);
        println!("Hello from second_task at CPU#{}", which_cpu());
        trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
    }
}

#[no_mangle]
pub extern "C" fn KHello_nobsp() {
    loop {
        // busy_delay(1);
        println!("Hello from KHello_nobsp() at CPU{}", which_cpu());
        trapping(S2Mop::TEST, &[0, 0, 0, 0, 0]);
        unsafe {
            asm!("nop");
        }
    }
}
