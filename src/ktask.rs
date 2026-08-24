use crate::cpu::{busy_delay, which_cpu};
use crate::ecall::{trapping, S2Mop};
use crate::sem_uart;
use crate::IRQ_BUFFER;
use crate::{Sprintln, S_UART};

#[no_mangle]
pub extern "C" fn ktask_idle() {
    loop {
        busy_delay(1);
        let _ = trapping(S2Mop::YIELD, None);
    }
}

#[no_mangle]
pub extern "C" fn ktask_extint() {
    let cpuid = which_cpu();
    loop {
        unsafe {
            Sprintln!("ktask_extint() trying to blocked...");
            sem_uart.wait();
            Sprintln!("ktask_extint() unblocked");
            match IRQ_BUFFER.peek_req(cpuid) {
                Ok(Some(new_req)) => {
                    IRQ_BUFFER.dequeue_req(cpuid);

                    let hart = new_req.get_cpuid();
                    let extint_id = new_req.get_extint_id();
                    let data = new_req.get_data();

                    match extint_id {
                        10 => {
                            if let Some(ch) = data {
                                let ch = *ch as u8;
                                Sprintln!("Uart extint at CPU#{}: {}", hart, ch as char);
                            } else {
                                Sprintln!("Uart extint at CPU#{}: Failed", hart);
                            }
                        }
                        0 => {
                            // do nothing
                        }
                        _ => {
                            Sprintln!("Unsupported extint: #{} on CPU#{}", extint_id, hart);
                        }
                    }
                }
                Ok(None) => {
                    // No interrupt pending — you can skip or log as needed
                }
                Err(e) => {
                    Sprintln!("Failed to peek IRQ on CPU#{}", cpuid);
                }
            }
        }
    }
    trapping(S2Mop::EXIT, None);
}

#[no_mangle]
pub extern "C" fn ktask_fallback() {
    trapping(S2Mop::YIELD, None);
}

#[no_mangle]
pub extern "C" fn paniker() {
    loop {}
}
