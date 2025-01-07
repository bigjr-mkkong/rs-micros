use crate::cpu::{busy_delay, set_cpu_mode, M_cli, M_sti, Mode, TrapFrame};
use crate::plic;
use crate::proc::{task_pool, task_struct};
use crate::EXTINT_SRCS;
use crate::SECALL_FRAME;
use crate::TASK_POOL;
use crate::{ecall_args, S2Mop};
use crate::{CLINT, PLIC};
use crate::{M_UART, S_UART};

use riscv::register;
use riscv::register::{mstatus, mstatus::MPP, sstatus, sstatus::SPP};

#[no_mangle]
extern "C" fn s_trap(
    xepc: usize,
    xtval: usize,
    xcause: usize,
    hart: usize,
    xstatus: usize,
    frame: &mut TrapFrame,
) -> usize {
    set_cpu_mode(Mode::Supervisor, hart);
    let spp: Mode = sstatus::read().spp().into();

    let is_async = if xcause >> 63 & 1 == 1 { true } else { false };

    let cause_num = xcause & 0xfff;
    let mut pc_ret = xepc;

    if is_async {
        match cause_num {
            3 => {
                Mprintln!("Supervisor: SW Interrupt at CPU#{}", hart);
            }
            9 => {
                Mprintln!("Supervisor: Ext Interrupt at CPU#{}", hart);
            }
            _ => {
                panic!("S-mode: Unhandled async trap on CPU#{}", hart);
            }
        }
    } else {
        Mprintln!("This exception should not been handled at S-mode");
        panic!();
    }

    set_cpu_mode(spp, hart);

    pc_ret
}

#[no_mangle]
extern "C" fn m_trap(
    xepc: usize,
    xtval: usize,
    xcause: usize,
    hart: usize,
    xstatus: usize,
    frame: &mut TrapFrame,
) -> usize {
    set_cpu_mode(Mode::Machine, hart);
    let mpp: Mode = mstatus::read().mpp().into();

    let is_async = if xcause >> 63 & 1 == 1 { true } else { false };

    let cause_num = xcause & 0xfff;
    let mut pc_ret: usize = xepc;

    let mut cdump_flag: bool = false;

    if is_async {
        match cause_num {
            3 => {
                Mprintln!("Machine SW Interrupt at CPU#{}", hart);
            }
            7 => {
                Mprintln!("Machine Timer Interrupt at CPU#{}", hart);
                unsafe {
                    CLINT.set_mtimecmp(hart, CLINT.read_mtime() + 0x500_000);
                }
            }
            11 => {
                unsafe {
                    let current_ctx = plic::id2plic_ctx(hart);
                    let extint_id = PLIC.claim(&current_ctx).unwrap_or(60);
                    match extint_id {
                        10 => {
                            let ch_get = M_UART.lock().get();
                            if let Some(ch) = ch_get {
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
                    PLIC.complete(&current_ctx, extint_id);
                }
            }
            _ => {
                Mprintln!("Unhandled async trap on CPU#{}", hart);
                cdump_flag = true;
            }
        }
    } else {
        match cause_num {
            0 => {
                Mprintln!("Instruction Address Misaligned at CPU#{}\n", hart);
                cdump_flag = true;
            }
            1 => {
                Mprintln!("Instruction Access Fault at CPU#{}\n", hart);
                cdump_flag = true;
            }
            2 => {
                Mprintln!("Illegal instruction at CPU#{}\n", hart);
                cdump_flag = true;
            }
            3 => {
                // Mprintln!("Breakpoint Trap at CPU#{}\n", hart);
                pc_ret += 4;
            }
            4 => {
                Mprintln!("Load Address Misaligned at CPU#{}\n", hart);
                cdump_flag = true;
            }
            5 => {
                Mprintln!("Load Access Fault at CPU#{}\n", hart);
                cdump_flag = true;
            }
            6 => {
                Mprintln!("Store/AMO Address Misaligned at CPU#{}\n", hart);
                cdump_flag = true;
            }
            7 => {
                Mprintln!("Store/AMO Access Fault at CPU#{}\n", hart);
                cdump_flag = true;
            }
            8 => {
                Mprintln!("E-call from User mode at CPU#{}", hart);
                pc_ret += 4;
            }
            9 => {
                // Mprintln!("E-call from Supervisor mode at CPU#{}", hart);
                unsafe {
                    let opcode = SECALL_FRAME[hart].get_opcode();
                    match opcode {
                        S2Mop::UNDEF => {
                            panic!("Supervisor is tring to call undefined operation");
                        }
                        S2Mop::TEST => {
                            TASK_POOL.save_from_ktrapframe(hart);
                            TASK_POOL.set_currentPC(hart, pc_ret + 4);
                            TASK_POOL.sched(hart);
                        }
                    }
                }
                pc_ret += 4;
            }
            11 => {
                Mprintln!("E-call from Machine mode at CPU#{}\n", hart);
            }
            12 => {
                Mprintln!("Instruction page fault at CPU#{}", hart);
                cdump_flag = true;
            }
            13 => {
                Mprintln!("Load page fault at CPU#{}", hart);
                cdump_flag = true;
            }
            15 => {
                Mprintln!("Store page fault at CPU#{}", hart);
                cdump_flag = true;
            }
            _ => {
                Mprintln!("Unhandled sync trap at CPU#{}\n", hart);
                cdump_flag = true;
            }
        }

        if cdump_flag == true {
            Mprintln!(
                "
>>>>>>Core Dump<<<<<<
---------------------
CPU {} 
Trapped instruction: 0x{:x}
xtval: 0x{:x}
xstatus: 0x{:x}
satp: 0x{:x}
---------------------",
                hart,
                xepc,
                xtval,
                xstatus,
                cpu::satp_read()
            );
            panic!();
        }
    }

    set_cpu_mode(mpp, hart);

    pc_ret
}
