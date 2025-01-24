use core::arch::asm;
use core::mem::size_of;
use core::ptr;
use riscv::register::{medeleg, mideleg, mie, mstatus, sie, sstatus};
use spin::Mutex;

use crate::cpu::{which_cpu, SATP_mode, TrapFrame};
use crate::error::{KError, KErrorType};
use crate::ktask::{KHello_task0, KHello_task1};
use crate::lock::spin_mutex;
use crate::lock::{M_lock, S_lock};
use crate::page;
use crate::plic::{extint_name, extint_src, plic_controller, plic_ctx};
use crate::proc::{task_struct, task_flag};
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

    /*
     * Now we only consider sw interrupt, timer and external
     * interrupt will be enabled in future
     *
     * We will delegate all interrupt into S-mode, enable S-mode
     * interrupt, and then disable M-mode interrupt
     */

    unsafe {
        mstatus::set_sie();
        CLINT.set_mtimecmp(current_cpu, u64::MAX);

        mie::set_msoft();

        // mie::set_mtimer();

        mie::set_mext();

        mie::set_sext();
        sstatus::set_spie();
        sie::set_sext();

        mstatus::set_mpp(mstatus::MPP::Supervisor);
    }

    cpu::sfence_vma();

    Ok(0)
}

pub fn kmain() -> Result<(), KError> {
    let current_cpu = which_cpu();
    Sprintln!("CPU#{} Switched to S mode", current_cpu);

    unsafe {
        asm!("ebreak");

        Sprintln!("CPU{} Back from trap\n", current_cpu);
        CLINT.set_mtimecmp(current_cpu, CLINT.read_mtime() + 0x500_000);

        let mut khello_task: task_struct = task_struct::new();
        let mut second_task_pcb: task_struct = task_struct::new();
        khello_task.init(KHello_task0 as usize, task_flag::NORMAL);
        second_task_pcb.init(KHello_task1 as usize, task_flag::NORMAL);

        KTHREAD_POOL.append_task(khello_task, which_cpu());
        KTHREAD_POOL.append_task(second_task_pcb, which_cpu());
        KTHREAD_POOL.sched(which_cpu());
    }

    loop {
        Sprintln!("CPU#{} kmain keep running...", current_cpu);
        let _ = cpu::busy_delay(1);
        unsafe {
            asm!("nop");
        }
    }
}
