use crate::cpu::{TrapFrame, M_cli, M_sti, set_cpu_mode, Mode};
use crate::{M_UART, S_UART};
use crate::CLINT;
use crate::SECALL_FRAME;
use crate::{ecall_args, S2Mop};
use riscv::register;
use riscv::register::{mstatus, sstatus, sstatus::SPP, mstatus::MPP};

#[no_mangle]
extern "C"
fn s_trap(xepc: usize, 
        xtval: usize,
        xcause: usize,
        hart: usize,
        xstatus: usize,
        frame: &mut TrapFrame) -> usize{
        
    set_cpu_mode(Mode::Supervisor, hart);
    let spp:Mode = sstatus::read().spp().into();

    let is_async = if xcause >> 63 & 1 == 1 { true } else {false};

    let cause_num = xcause & 0xfff;
    let mut pc_ret = xepc;

    if is_async{
        match cause_num{
            3 => {
                println!("Supervisor: SW Interrupt at CPU#{}", hart);
            },
            11 => {
                println!("Supervisor: External Interrupt at CPU#{}", hart);
                panic!("Panic for test reason...");
            },
            _ => {
                panic!("S-mode: Unhandled async trap on CPU#{}", hart);
            }
        }
    }else{
        println!("This exception should not been handled at S-mode");
        panic!();
    }

    set_cpu_mode(spp, hart);

    pc_ret
}

#[no_mangle]
extern "C"
fn m_trap(xepc: usize, 
        xtval: usize,
        xcause: usize,
        hart: usize,
        xstatus: usize,
        frame: &mut TrapFrame) -> usize{
        
    set_cpu_mode(Mode::Machine, hart);
    let mpp:Mode = mstatus::read().mpp().into();

    let is_async = if xcause >> 63 & 1 == 1 { true } else {false};

    let cause_num = xcause & 0xfff;
    let mut pc_ret = xepc;

    if is_async{
        match cause_num{
            3 => {
                println!("Machine SW Interrupt at CPU#{}", hart);
            },
            7 => {
                println!("Machine Timer Interrupt at CPU#{}", hart);
                unsafe{
                    CLINT.set_mtimecmp(hart, CLINT.read_mtime() + 0x500_000);
                }
            },
            11 => {
                println!("Machine External Interrupt at CPU#{}", hart);
                panic!("Panic for test reason...");
            },
            _ => {
                panic!("Unhandled async trap on CPU#{}", hart);
            }
        }
    }else{
        match cause_num {
            0 => {
				println!("Instruction Address Misaligned at CPU#{}\n", hart);
                panic!();
            },
            1 => {
				println!("Instruction Access Fault at CPU#{}\n", hart);
                panic!();
            },
			2 => {
				println!("Illegal instruction at CPU#{}\n", hart);
                panic!();
			},
			3 => {
				println!("Breakpoint Trap at CPU#{}\n", hart);
                pc_ret += 4;
			},
			4 => {
				println!("Load Address Misaligned at CPU#{}\n", hart);
                panic!();
			},
			5 => {
				println!("Load Access Fault at CPU#{}\n", hart);
                panic!();
			},
			6 => {
				println!("Store/AMO Address Misaligned at CPU#{}\n", hart);
                panic!();
			},
			7 => {
				println!("Store/AMO Access Fault at CPU#{}\n", hart);
                panic!();
			},
			8 => {
				println!("E-call from User mode at CPU#{}", hart);
				pc_ret += 4;
			},
			9 => {
				println!("E-call from Supervisor mode at CPU#{}", hart);
                unsafe{
                    let opcode = SECALL_FRAME[hart].get_opcode();
                    match opcode{
                        S2Mop::CLI => {
                            let cli_ret = M_cli();
                            SECALL_FRAME[hart].set_ret(cli_ret);
                        },
                        S2Mop::STI => {
                            let prev_mie = SECALL_FRAME[hart].get_args()[0];
                            M_sti(prev_mie);
                        },
                        S2Mop::UNDEF => {
                            panic!("Supervisor is tring to call undefined operation");
                        }
                    }
                }
				pc_ret += 4;
			},
			11 => {
				println!("E-call from Machine mode at CPU#{}\n", hart);
			},
			12 => {
				println!("Instruction page fault at CPU#{}", hart);
			},
			13 => {
				println!("Load page fault at CPU#{}", hart);
			},
			15 => {
				println!("Store page fault at CPU#{}", hart);
				pc_ret += 4;
			},
			_ => {
				println!("Unhandled sync trap at CPU#{}\n", hart);
                panic!();
			}
		}
        /*
         * TODO
         * Implement reg_dump on print trap CSRs
         */
        // reg_dump();
    }

    set_cpu_mode(mpp, hart);

    pc_ret
}
