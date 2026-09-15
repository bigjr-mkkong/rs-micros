use crate::cpu::{self, set_cpu_mode, Mode, S_cli, S_sti, TrapFrame};
use crate::irq::{int_request, int_type};
use crate::kthread::{task_flag, task_state, INVAL_KTHREADS_PID};
use crate::plic;
use crate::{Mprintln, Sprintln};
use crate::{S2Mop, M_UART, S_UART};
use crate::{CLINT, IRQ_BUFFER, KERNEL_TRAP_FRAME, KTHREAD_POOL, PLIC, SECALL_FRAME};

use core::sync::atomic::{AtomicUsize, Ordering};
use riscv::register::{medeleg, mideleg, mie, mstatus, sstatus};

const MACHINE_TIMER_INTERVAL: u64 = 0x500_000;
static MACHINE_TIMER_TICKS: [AtomicUsize; cpu::MAX_HARTS] =
    [const { AtomicUsize::new(0) }; cpu::MAX_HARTS];

/// Route every supervisor-capable interrupt class and every supported S/U
/// exception directly to `stvec` on the current hart.
///
/// Machine-level timer/software/external interrupts cannot be delegated. The
/// runtime uses the supervisor PLIC context for external interrupts, while
/// MTIP and supervisor ecalls deliberately remain M-mode firmware services.
pub unsafe fn delegate_traps_to_supervisor() {
    mideleg::set_ssoft();
    mideleg::set_sext();

    medeleg::set_instruction_misaligned();
    medeleg::set_instruction_fault();
    medeleg::set_illegal_instruction();
    medeleg::set_breakpoint();
    medeleg::set_load_misaligned();
    medeleg::set_load_fault();
    medeleg::set_store_misaligned();
    medeleg::set_store_fault();
    medeleg::set_user_env_call();
    medeleg::set_instruction_page_fault();
    medeleg::set_load_page_fault();
    medeleg::set_store_page_fault();
}

/// Arm the hart-local machine timer. This must run once on every hart while it
/// is still in M-mode because `mie` is a hart-local CSR.
pub unsafe fn enable_machine_timer(hart: usize) {
    CLINT.set_mtimecmp(hart, CLINT.read_mtime() + MACHINE_TIMER_INTERVAL);
    mie::set_mtimer();
}

pub fn machine_timer_ticks(hart: usize) -> usize {
    MACHINE_TIMER_TICKS
        .get(hart)
        .map(|ticks| ticks.load(Ordering::Acquire))
        .unwrap_or(0)
}

fn breakpoint_resume_pc(pc: usize) -> usize {
    // C.EBREAK is 16 bits; the regular EBREAK encoding is 32 bits.
    let instruction = unsafe { (pc as *const u16).read_volatile() };
    pc + if instruction & 0b11 == 0b11 { 4 } else { 2 }
}

fn supervisor_core_dump(
    description: &str,
    hart: usize,
    xepc: usize,
    xtval: usize,
    xstatus: usize,
) -> ! {
    Sprintln!(
        "S-mode {} at CPU#{}\n\
         >>>>>>Core Dump<<<<<<\n\
         ---------------------\n\
         xepc: 0x{:x}\n\
         xtval: 0x{:x}\n\
         xstatus: 0x{:x}\n\
         satp: 0x{:x}\n\
         ---------------------",
        description,
        hart,
        xepc,
        xtval,
        xstatus,
        cpu::satp_read()
    );
    panic!("fatal S-mode trap");
}

unsafe fn supervisor_external_interrupt(hart: usize) {
    let current_ctx = plic::id2plic_ctx(hart);
    let extint_id = PLIC.claim(&current_ctx).unwrap_or(0);

    // A claim value of zero means another enabled target won the claim or no
    // interrupt remains pending. There is nothing to complete in that case.
    if extint_id == 0 {
        return;
    }

    let data = match extint_id {
        10 => S_UART.lock().get().map(|ch| ch as usize),
        _ => None,
    };

    PLIC.complete(&current_ctx, extint_id)
        .unwrap_or_else(|_| panic!("failed to complete PLIC interrupt"));

    if extint_id != 10 {
        panic!("Unsupported extint: #{} on CPU#{}", extint_id, hart);
    }

    if !IRQ_BUFFER.is_full(hart).unwrap_or(true) {
        let mut new_irq_req = int_request::new();
        new_irq_req.set_typ(int_type::EXTERNAL);
        new_irq_req.set_extint_id(extint_id);
        new_irq_req.set_cpuid(hart);
        new_irq_req.set_data(data);

        IRQ_BUFFER
            .push_req(new_irq_req, hart)
            .unwrap_or_else(|_| panic!("failed to queue external interrupt"));
        // We are already in the S-mode trap handler. Do not issue a nested
        // ecall to wake the waiter.
        crate::sem_uart.signal_from_trap();
    }
}

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
    let previous_mode: Mode = sstatus::read().spp().into();
    let is_async = xcause >> (usize::BITS - 1) == 1;
    let cause_num = xcause & 0xfff;
    let mut pc_ret = xepc;

    assert_eq!(
        frame.cpuid, hart,
        "S-mode trap entered with the wrong per-hart frame"
    );

    if is_async {
        match cause_num {
            1 => supervisor_core_dump(
                "software interrupt (no SSIP source is configured)",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            5 => supervisor_core_dump(
                "timer interrupt (no STIP source is configured)",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            9 => unsafe { supervisor_external_interrupt(hart) },
            _ => supervisor_core_dump("unhandled asynchronous trap", hart, xepc, xtval, xstatus),
        }
    } else {
        match cause_num {
            0 => supervisor_core_dump(
                "instruction-address-misaligned exception",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            1 => supervisor_core_dump(
                "instruction-access-fault exception",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            2 => supervisor_core_dump("illegal-instruction exception", hart, xepc, xtval, xstatus),
            3 => pc_ret = breakpoint_resume_pc(xepc),
            4 => supervisor_core_dump(
                "load-address-misaligned exception",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            5 => supervisor_core_dump("load-access-fault exception", hart, xepc, xtval, xstatus),
            6 => supervisor_core_dump(
                "store/AMO-address-misaligned exception",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            7 => supervisor_core_dump(
                "store/AMO-access-fault exception",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            8 => {
                Sprintln!("E-call from User mode at CPU#{}", hart);
                pc_ret += 4;
            }
            9 => {
                supervisor_core_dump(
                    "unexpected delegated supervisor ecall",
                    hart,
                    xepc,
                    xtval,
                    xstatus,
                );
            }
            12 => supervisor_core_dump(
                "instruction-page-fault exception",
                hart,
                xepc,
                xtval,
                xstatus,
            ),
            13 => supervisor_core_dump("load-page-fault exception", hart, xepc, xtval, xstatus),
            15 => supervisor_core_dump("store-page-fault exception", hart, xepc, xtval, xstatus),
            _ => supervisor_core_dump("unhandled synchronous trap", hart, xepc, xtval, xstatus),
        }
    }

    set_cpu_mode(previous_mode, hart);
    pc_ret
}

#[no_mangle]
extern "C" fn m_trap(
    xepc: usize,
    xtval: usize,
    xcause: usize,
    hart: usize,
    xstatus: usize,
    _frame: &mut TrapFrame,
) -> usize {
    set_cpu_mode(Mode::Machine_IRH, hart);
    let previous_mode: Mode = mstatus::read().mpp().into();
    let is_async = xcause >> (usize::BITS - 1) == 1;
    let cause_num = xcause & 0xfff;

    // MTIP and supervisor ecalls form the intentionally retained M-mode
    // firmware boundary. Everything else at runtime should have been routed
    // to the S-mode handler.
    if is_async && cause_num == 7 {
        unsafe {
            CLINT.set_mtimecmp(hart, CLINT.read_mtime() + MACHINE_TIMER_INTERVAL);
        }
        MACHINE_TIMER_TICKS[hart].fetch_add(1, Ordering::Release);
        set_cpu_mode(previous_mode, hart);
        return xepc;
    }

    if !is_async && cause_num == 9 {
        machine_ecall_handler(xepc, hart);
        set_cpu_mode(previous_mode, hart);
        return xepc + 4;
    }

    // Keep breakpoints usable while booting.
    if !is_async && cause_num == 3 {
        let pc_ret = breakpoint_resume_pc(xepc);
        set_cpu_mode(previous_mode, hart);
        return pc_ret;
    }

    Mprintln!(
        "Unexpected M-mode {} trap #{} at CPU#{}\n\
         xepc: 0x{:x}\n\
         xtval: 0x{:x}\n\
         xstatus: 0x{:x}\n\
         satp: 0x{:x}",
        if is_async { "interrupt" } else { "exception" },
        cause_num,
        hart,
        xepc,
        xtval,
        xstatus,
        cpu::satp_read()
    );
    panic!("runtime trap reached M-mode");
}

fn restore_critical_interrupts(hart: usize) {
    unsafe {
        if let Ok(task_flag::CRITICAL) = KTHREAD_POOL.get_current_fg(hart) {
            let previous_sie = KTHREAD_POOL.get_crit_task_mie()[hart];
            S_sti(previous_sie);
        }
    }
}

fn machine_ecall_handler(pc: usize, hart: usize) {
    unsafe {
        let opcode = SECALL_FRAME[hart].get_opcode();
        match opcode {
            S2Mop::UNDEF => panic!("Supervisor is trying to call an undefined operation"),
            S2Mop::YIELD => {
                restore_critical_interrupts(hart);

                if KTHREAD_POOL.is_in_fallback(hart) {
                    // M traps save into KERNEL_TRAP_FRAME, not directly into
                    // the fallback frame. Preserve its live return address
                    // and stack before resuming after the ecall.
                    KTHREAD_POOL
                        .save_fallback_from_ktrapframe(hart)
                        .unwrap_or_else(|_| panic!("failed to save fallback task context"));
                    KTHREAD_POOL
                        .set_fallback_pc(hart, pc + 4)
                        .unwrap_or_else(|_| panic!("failed to save fallback task PC"));
                } else {
                    KTHREAD_POOL
                        .save_from_ktrapframe(hart)
                        .unwrap_or_else(|_| panic!("failed to save yielded task context"));
                    KTHREAD_POOL
                        .set_currentPC(hart, pc + 4)
                        .unwrap_or_else(|_| panic!("failed to save yielded task PC"));
                }

                let _ = KTHREAD_POOL.sched(hart);
                KTHREAD_POOL
                    .fallback(hart)
                    .unwrap_or_else(|_| panic!("failed to schedule fallback task"));
            }
            S2Mop::EXIT => {
                restore_critical_interrupts(hart);
                KTHREAD_POOL
                    .remove_cur_task(hart)
                    .unwrap_or_else(|_| panic!("failed to remove exiting task"));
                let _ = KTHREAD_POOL.sched(hart);
                KTHREAD_POOL
                    .fallback(hart)
                    .unwrap_or_else(|_| panic!("failed to schedule fallback task"));
            }
            S2Mop::BLOCK => {
                restore_critical_interrupts(hart);
                KTHREAD_POOL
                    .save_from_ktrapframe(hart)
                    .unwrap_or_else(|_| panic!("failed to save blocked task context"));
                KTHREAD_POOL
                    .set_currentPC(hart, pc + 4)
                    .unwrap_or_else(|_| panic!("failed to save blocked task PC"));

                let _ = KTHREAD_POOL.sched(hart);
                KTHREAD_POOL
                    .fallback(hart)
                    .unwrap_or_else(|_| panic!("failed to schedule fallback task"));
            }
            S2Mop::UNBLOCK => {
                let args = SECALL_FRAME[hart].get_args();
                let target_pid = args[0];
                let target_lifeid = args[1];
                assert_ne!(target_pid, INVAL_KTHREADS_PID);
                assert_ne!(target_lifeid, 0);

                if !KTHREAD_POOL.is_in_fallback(hart) {
                    KTHREAD_POOL
                        .save_from_ktrapframe(hart)
                        .unwrap_or_else(|_| panic!("failed to save signaling task context"));
                    KTHREAD_POOL
                        .set_currentPC(hart, pc + 4)
                        .unwrap_or_else(|_| panic!("failed to save signaling task PC"));
                }
                KTHREAD_POOL
                    .set_state_by_pid(target_pid, target_lifeid, task_state::Ready)
                    .unwrap_or_else(|_| panic!("failed to unblock task"));
            }
            S2Mop::CLI => {
                let previous_sie = S_cli();
                KERNEL_TRAP_FRAME[hart].mie_buf = previous_sie;
            }
            S2Mop::STI => {
                let previous_sie = KERNEL_TRAP_FRAME[hart].mie_buf;
                S_sti(previous_sie);
            }
        }
    }
}
