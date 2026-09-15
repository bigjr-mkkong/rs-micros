use crate::{Mprintln, Sprintln};
use core::arch::asm;
use core::mem::size_of;
use core::ptr;
use riscv::register::{mstatus, sie, sstatus};
use spin::Mutex;

use crate::cpu::{which_cpu, SATP_mode, TrapFrame};
use crate::error::{KError, KErrorType};
use crate::ktask::ktask_idle;
use crate::kthread::{task_flag, task_struct};
use crate::lock::spin_mutex;
use crate::lock::{M_lock, S_lock};
use crate::page;
use crate::plic::{extint_name, extint_src, plic_controller, plic_ctx};
use crate::vm::{ident_range_map, virt2phys};
use crate::zone::{kfree_page, kmalloc_page, zone_type};
use crate::CLINT;
use crate::KTHREAD_POOL;

use crate::{cpu, kmem, vm, KERNEL_TRAP_FRAME, M_UART, S_UART};

pub fn kinit() -> Result<usize, KError> {
    let current_cpu = which_cpu();

    Mprintln!("CPU#{} is running its nobsp_kinit()", current_cpu);

    let pageroot_ptr = kmem::get_page_table();
    let mut pageroot = unsafe { pageroot_ptr.as_mut().unwrap() };

    cpu::satp_write(SATP_mode::Sv39, 0, pageroot_ptr as usize);

    cpu::mepc_write(crate::eh_func_nobsp_kmain as usize);

    // cpu::mstatus_write((1 << 11) | (1 << 5) as usize);

    // Delegation CSRs are hart-local, so every secondary must install the
    // same S-mode trap policy as the BSP.

    unsafe {
        crate::trap::delegate_traps_to_supervisor();
        crate::trap::enable_machine_timer(current_cpu);
        sie::set_sext();
        sstatus::set_sie();
        sstatus::set_spie();

        mstatus::set_mpp(mstatus::MPP::Supervisor);
    }

    cpu::flush_tlb();

    Ok(0)
}

pub fn kmain() -> Result<(), KError> {
    let current_cpu = which_cpu();
    Sprintln!("CPU#{} Switched to S mode", current_cpu);

    unsafe {
        asm!("ebreak");

        Sprintln!("CPU{} Back from trap\n", current_cpu);

        let sched_cpu = which_cpu();

        #[cfg(feature = "multicore-test")]
        crate::multicore_test::start(sched_cpu, false)?;

        #[cfg(not(feature = "multicore-test"))]
        {
            // Keep secondary schedulers active without competing for UART IRQ
            // requests, which are routed to hart 0.
            KTHREAD_POOL.spawn(ktask_idle as usize, task_flag::NORMAL, sched_cpu)?;
            KTHREAD_POOL.join_all_ktask(sched_cpu);
        }
    }

    loop {
        Sprintln!("CPU#{} kmain keep running...", current_cpu);
        let _ = cpu::busy_delay(1);
        unsafe {
            asm!("nop");
        }
    }
}
