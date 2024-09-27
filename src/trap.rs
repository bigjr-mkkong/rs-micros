use crate::cpu::TrapFrame;
use crate::SYS_UART;

#[no_mangle]
extern "C"
fn s_trap(xepc: usize, 
        xtval: usize,
        xcause: usize,
        hart: usize,
        xstatus: usize,
        frame: &mut TrapFrame) -> usize{
        

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
            },
            11 => {
                println!("Machine External Interrupt at CPU#{}", hart);
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
                println!("Will Panic for test reason");
                panic!();
            },
            11 => {
                println!("Machine External Interrupt at CPU#{}", hart);
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

    pc_ret
}
