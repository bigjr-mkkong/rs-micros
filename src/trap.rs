use crate::cpu::{busy_delay, set_cpu_mode, M_cli, M_sti, Mode, TrapFrame};
use crate::irq::{int_request, int_type};
use crate::ktask::ktask_extint;
use crate::kthread::{task_flag, task_pool, task_state, task_struct};
use crate::plic;
use crate::sem_uart;
use crate::EXTINT_SRCS;
use crate::IRQ_BUFFER;
use crate::KERNEL_TRAP_FRAME;
use crate::KTHREAD_POOL;
use crate::SECALL_FRAME;
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
    set_cpu_mode(Mode::Machine_IRH, hart);
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
                    let mut data: Option<usize> = None;
                    match extint_id {
                        10 => {
                            let ch_get = M_UART.lock().get();
                            if let Some(ch) = ch_get {
                                data = Some(ch as usize);
                            } else {
                                data = None;
                            }
                        }
                        0 => {
                            //do nothing when 0
                        }
                        _ => {
                            panic!("Unsupported extint: #{} on CPU#{}", extint_id, hart);
                        }
                    }

                    PLIC.complete(&current_ctx, extint_id);

                    if let Ok(is_full) = IRQ_BUFFER.is_full(hart) {
                        if !is_full {
                            let mut new_irq_req: int_request = int_request::new();

                            new_irq_req.set_typ(int_type::EXTERNAL);
                            new_irq_req.set_extint_id(extint_id);
                            new_irq_req.set_cpuid(hart);
                            new_irq_req.set_data(data);

                            IRQ_BUFFER.push_req(new_irq_req, hart);
                            sem_uart.signal(Some(hart));
                        }
                    }
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
                ecall_handler(pc_ret, hart);
                pc_ret += 4;
            }
            11 => {
                Mprintln!("E-call from Machine mode at CPU#{}\n", hart);
                ecall_handler(pc_ret, hart);
                pc_ret += 4;
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
xepc: 0x{:x}
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

fn ecall_handler(pc_ret: usize, hart: usize) {
    unsafe {
        let opcode = SECALL_FRAME[hart].get_opcode();
        match opcode {
            S2Mop::UNDEF => {
                panic!("Supervisor is tring to call undefined operation");
            }
            S2Mop::YIELD => {
                if let Ok(task_flag::CRITICAL) = KTHREAD_POOL.get_current_fg(hart) {
                    let prev_mie = KTHREAD_POOL.get_crit_task_mie();
                    M_sti(prev_mie[hart]);
                }
                KTHREAD_POOL.save_from_ktrapframe(hart);
                KTHREAD_POOL.set_currentPC(hart, pc_ret + 4);
                KTHREAD_POOL.sched(hart);
            }
            S2Mop::EXIT => {
                KTHREAD_POOL.remove_cur_task(hart);
                KTHREAD_POOL.sched(hart);
                KTHREAD_POOL.fallback(hart);
            }
            S2Mop::BLOCK => {
                if let Ok(task_flag::CRITICAL) = KTHREAD_POOL.get_current_fg(hart) {
                    let prev_mie = KTHREAD_POOL.get_crit_task_mie();
                    M_sti(prev_mie[hart]);
                }
                let args = SECALL_FRAME[hart].get_args();
                let target_pid = args[0];

                KTHREAD_POOL.save_from_ktrapframe(hart);
                KTHREAD_POOL.set_currentPC(hart, pc_ret + 4);

                KTHREAD_POOL.set_state_by_pid(target_pid, task_state::Block);

                KTHREAD_POOL.sched(hart);
            }
            S2Mop::UNBLOCK => {
                let args = SECALL_FRAME[hart].get_args();
                let target_pid = args[0];

                KTHREAD_POOL.save_from_ktrapframe(hart);
                KTHREAD_POOL.set_currentPC(hart, pc_ret + 4);

                KTHREAD_POOL.set_state_by_pid(target_pid, task_state::Ready);
            }
            S2Mop::CLI => {
                let prev_mie = M_cli();
                KERNEL_TRAP_FRAME[hart].mie_buf = prev_mie;
            }
            S2Mop::STI => {
                let prev_mie = KERNEL_TRAP_FRAME[hart].mie_buf;
                M_sti(prev_mie);
            }
        }
    }
}
