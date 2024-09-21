
use core::arch::asm;
use core::mem::size_of;
use core::ptr;
use spin::Mutex;
use riscv::register::{mideleg, medeleg};

use crate::error::{KError, KErrorType};
use crate::zone::{zone_type, kmalloc_page, kfree_page};
use crate::page;
use crate::vm::{ident_range_map, virt2phys};
use crate::cpu::{SATP_mode, TrapFrame, irq_mutex, which_cpu};
use crate::plic::{plic_controller, plic_ctx, extint_map};

use crate::{kmem,
            vm,
            cpu,
            SYS_UART,
            KERNEL_TRAP_FRAME,};

pub fn kinit() -> Result<usize, KError> {
    let current_cpu = which_cpu();

    println!("CPU#{} is running its nobsp_kinit()", current_cpu);

    let pageroot_ptr = kmem::get_page_table();
    let mut pageroot = unsafe{pageroot_ptr.as_mut().unwrap()};

    unsafe{
        cpu::sscratch_write((&mut KERNEL_TRAP_FRAME[0] as *mut TrapFrame) as usize);

        KERNEL_TRAP_FRAME[current_cpu].trap_stack = 
                kmalloc_page(zone_type::ZONE_NORMAL, 1)?.add(page::PAGE_SIZE);

        ident_range_map(pageroot, 
                KERNEL_TRAP_FRAME[current_cpu].trap_stack.sub(page::PAGE_SIZE) as usize,
                KERNEL_TRAP_FRAME[current_cpu].trap_stack as usize,
                vm::EntryBits::ReadWrite.val());

        let trapstack_paddr = KERNEL_TRAP_FRAME[current_cpu].trap_stack as usize - 1;
        let trapstack_vaddr = virt2phys(&pageroot, trapstack_paddr)?.unwrap_or(0);

        println!("CPU#{} TrapStack: (vaddr){:#x} -> (paddr){:#x}", 
                current_cpu,
                trapstack_paddr, 
                trapstack_vaddr
                );

        let trapfram_paddr = ptr::addr_of_mut!(KERNEL_TRAP_FRAME[current_cpu]) as usize;
        let trapfram_vaddr = virt2phys(&pageroot, trapfram_paddr)?.unwrap_or(0);

        println!("CPU#{} TrapFrame: (vaddr){:#x} -> (paddr){:#x}", 
                current_cpu,
                trapfram_paddr, 
                trapfram_vaddr
                );
    }

    cpu::satp_write(SATP_mode::Sv39, 0, pageroot_ptr as usize);

    cpu::mepc_write(crate::eh_func_nobsp_kmain as usize);

    cpu::mstatus_write((1 << 11) | (1 << 5) as usize);

    /*
     * Now we only consider sw interrupt, timer and external
     * interrupt will be enabled in future
     *
     * We will delegate all interrupt into S-mode, enable S-mode
     * interrupt, and then disable M-mode interrupt
     */

    unsafe{
        mideleg::set_sext();
        mideleg::set_ssoft();
        mideleg::set_stimer();

        let all_exception:usize = 0xffffffff;
        asm!("csrw medeleg, {0}", in(reg) all_exception);
    }

    cpu::mie_write(0 as usize);

    cpu::sie_write((1 << 3) as usize);
    
    cpu::sfence_vma();

    
    Ok(0)
}

pub fn kmain() -> Result<(), KError> {
    let current_cpu = which_cpu();
    println!("CPU#{} Switched to S mode", current_cpu);

    unsafe{
        asm!("ebreak");
    }

    loop{
        let ch_ops = SYS_UART.lock().get();
        match ch_ops {
            Some(ch) => {
                println!("{}", ch as char);
            },
            None => {}
        }
        unsafe{
            asm!("nop");
        }
    }
}
