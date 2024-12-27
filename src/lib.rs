#![no_std]
#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(variant_count)]

extern crate alloc;

extern "C" {
    static _heap_start: u8;
    static _heap_end: u8;

    static _stack_start: u8;
    static _stack_end: u8;

    static _text_start: u8;
    static _text_end: u8;

    static _rodata_start: u8;
    static _rodata_end: u8;

    static _data_start: u8;
    static _data_end: u8;

    static _bss_start: u8;
    static _bss_end: u8;

    static _virtio_start: u8;
    static _virtio_end: u8;

    static mut cpu_early_block: u64;
}

use core::arch::asm;
use core::mem::size_of;
use core::ptr;
use riscv::register::{medeleg, mideleg, mie, mstatus, sie, sstatus};
use spin::Mutex;

use crate::lock::spin_mutex;
use crate::lock::{M_lock, S_lock};
use clint::clint_controller;
use cpu::{get_cpu_mode, which_cpu, SATP_mode, TrapFrame};
use ecall::{ecall_args, S2Mop};
use error::{KError, KErrorType};
use nobsp_kfunc::kinit as nobsp_kinit;
use nobsp_kfunc::kmain as nobsp_kmain;
use plic::{extint_name, extint_src, plic_controller, plic_ctx};
use proc::{task_struct, task_pool};
use vm::{ident_range_map, virt2phys};
use zone::{kfree_page, kmalloc_page, zone_type};
use alloc::vec::Vec;

#[macro_export]
macro_rules! print
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        use crate::cpu;
        if let cpu::Mode::Machine = cpu::get_cpu_mode(cpu::which_cpu()) {
            let _ = write!(M_UART.lock(), $($args)+);
        }else{
            let _ = write!(S_UART.lock(), $($args)+);
        }
    });
}

#[macro_export]
macro_rules! println
{
    () => ({
        print("\r\n")
    });

    ($fmt:expr) => ({
        print!(concat!($fmt, "\r\n"))
    });

    ($fmt:expr, $($args:tt)+) => ({
        print!(concat!($fmt, "\r\n"), $($args)+)
    });

}

#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    print!("System Aborting...");
    if let Some(p) = info.location() {
        println!("line {}, file {}: {}", p.line(), p.file(), info.message());
    } else {
        println!("PanicInfo not available yet");
    }

    abort();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            asm!("nop");
        }
    }
}

#[no_mangle]
extern "C" fn eh_func_kinit() -> usize {
    let cpuid = cpu::mhartid_read();
    unsafe {
        cpu::mscratch_write((&mut KERNEL_TRAP_FRAME[cpuid] as *mut TrapFrame) as usize);
        cpu::sscratch_write(cpu::mscratch_read());
    }
    cpu::set_cpu_mode(cpu::Mode::Machine, cpuid);
    let init_return = kinit();
    if let Err(er_code) = init_return {
        println!("{}", er_code);
        println!("kinit() Failed on CPU#{}, System halting now...", cpuid);
        loop {
            unsafe {
                asm!("nop");
            }
        }
    } else {
        init_return.unwrap_or_default()
    }
}

#[no_mangle]
extern "C" fn eh_func_kmain(cpuid: usize) {
    cpu::set_cpu_mode(cpu::Mode::Supervisor, cpuid);
    let main_return = kmain(cpuid);
    if let Err(er_code) = main_return {
        println!("{}", er_code);
        println!("kmain() Failed, System halting now...");
        loop {
            unsafe {
                asm!("nop");
            }
        }
    }
}

#[no_mangle]
extern "C" fn eh_func_kinit_nobsp() -> usize {
    let cpuid = cpu::mhartid_read();
    unsafe {
        cpu::mscratch_write((&mut KERNEL_TRAP_FRAME[cpuid] as *mut TrapFrame) as usize);
        cpu::sscratch_write(cpu::mscratch_read());
        KERNEL_TRAP_FRAME[cpuid].cpuid = cpuid;
    }
    cpu::set_cpu_mode(cpu::Mode::Machine, cpuid);
    let init_return = nobsp_kinit();
    if let Err(er_code) = init_return {
        println!("{}", er_code);
        println!(
            "nobsp_kinit() Failed at CPU#{}, System halting now...",
            cpuid
        );
        loop {
            unsafe {
                asm!("nop");
            }
        }
    } else {
        init_return.unwrap_or_default()
    }
}

#[no_mangle]
pub extern "C" fn eh_func_nobsp_kmain() {
    let main_return = nobsp_kmain();
    cpu::set_cpu_mode(cpu::Mode::Supervisor, which_cpu());
    if let Err(er_code) = main_return {
        println!("{}", er_code);
        println!("kmain() Failed, System halting now...");
        loop {
            unsafe {
                asm!("nop");
            }
        }
    }
}
//   ____ _     ___  ____    _    _      __     ___    ____  ____
//  / ___| |   / _ \| __ )  / \  | |     \ \   / / \  |  _ \/ ___|
// | |  _| |  | | | |  _ \ / _ \ | |      \ \ / / _ \ | |_) \___ \
// | |_| | |__| |_| | |_) / ___ \| |___    \ V / ___ \|  _ < ___) |
//  \____|_____\___/|____/_/   \_\_____|    \_/_/   \_\_| \_\____/
pub const ZONE_DEFVAL: spin_mutex<zone::mem_zone, S_lock> =
    spin_mutex::<zone::mem_zone, S_lock>::new(zone::mem_zone::new());

pub static SYS_ZONES: [spin_mutex<zone::mem_zone, S_lock>; 3] =
    [ZONE_DEFVAL; zone_type::type_cnt()];

pub static M_UART: spin_mutex<uart::Uart, M_lock> =
    spin_mutex::<uart::Uart, M_lock>::new(uart::Uart::new(0x1000_0000));

pub static S_UART: spin_mutex<uart::Uart, S_lock> =
    spin_mutex::<uart::Uart, S_lock>::new(uart::Uart::new(0x1000_0000));

pub static mut KERNEL_TRAP_FRAME: [TrapFrame; 8] = [TrapFrame::new(); 8];
pub static mut PLIC: plic_controller = plic_controller::new(plic::PLIC_BASE);
pub static mut CLINT: clint_controller = clint_controller::new(clint::CLINT_BASE);
pub static mut SECALL_FRAME: [ecall_args; cpu::MAX_HARTS] = [ecall_args::new(); cpu::MAX_HARTS];

pub static mut cust_hmalloc: spin_mutex<allocator::custom_kheap_malloc, S_lock> = spin_mutex::<
    allocator::custom_kheap_malloc,
    S_lock,
>::new(
    allocator::custom_kheap_malloc::new(),
);

#[global_allocator]
pub static glob_alloc: allocator::kheap_alloc = allocator::kheap_alloc::new();

pub static mut EXTINT_SRCS: [extint_src; plic::MAX_INTCNT] = [extint_src::new(); plic::MAX_INTCNT];

pub static mut TASK_POOL: task_pool = task_pool::new();


fn kinit() -> Result<usize, KError> {
    M_UART.lock().init();
    S_UART.lock().init();
    println!("\nHello world");

    let current_cpu = cpu::mhartid_read();
    println!("Initializer running on CPU#{}", current_cpu);

    /*
     * Setting up new zone
     */
    let (meta_begin, meta_end) = SYS_ZONES[zone_type::ZONE_NORMAL.val()].lock().init(
        ptr::addr_of!(_heap_start),
        ptr::addr_of!(_heap_end),
        zone_type::ZONE_NORMAL,
        zone::AllocatorSelector::NaiveAllocator,
    )?;

    SYS_ZONES[zone_type::ZONE_UNDEF.val()].lock().init(
        0 as *const u8,
        0 as *const u8,
        zone_type::ZONE_UNDEF,
        zone::AllocatorSelector::EmptyAllocator,
    )?;

    kmem::init()?;

    let pageroot_ptr = kmem::get_page_table();
    let mut pageroot = unsafe { pageroot_ptr.as_mut().unwrap() };

    let kheap_begin = kmem::get_kheap_start();
    let kheap_pgcnt = kmem::get_kheap_pgcnt();

    unsafe {
        cust_hmalloc
            .lock()
            .init(kheap_begin as usize, kheap_pgcnt * page::PAGE_SIZE);
    }

    ident_range_map(
        pageroot,
        aligl_4k!(ptr::addr_of!(_text_start) as usize),
        aligh_4k!(ptr::addr_of!(_text_end) as usize),
        vm::EntryBits::ReadExecute.val(),
    );

    // let usz_heap_start = ptr::addr_of!(_heap_start) as usize;
    // let usz_heap_end = usz_heap_start + SYS_ZONES[zone_type::ZONE_NORMAL.val()].lock().get_size()?;
    // ident_range_map(pageroot,
    //         usz_heap_start,
    //         usz_heap_end,
    //         vm::EntryBits::ReadWrite.val());

    ident_range_map(
        pageroot,
        kheap_begin as usize,
        kheap_begin as usize + page::PAGE_SIZE * kheap_pgcnt,
        vm::EntryBits::ReadWrite.val(),
    );

    ident_range_map(
        pageroot,
        aligl_4k!(ptr::addr_of!(_rodata_start) as usize),
        aligh_4k!(ptr::addr_of!(_rodata_end) as usize),
        vm::EntryBits::ReadExecute.val(),
    );

    ident_range_map(
        pageroot,
        aligl_4k!(ptr::addr_of!(_data_start) as usize),
        aligh_4k!(ptr::addr_of!(_data_end) as usize),
        vm::EntryBits::ReadWrite.val(),
    );

    ident_range_map(
        pageroot,
        aligl_4k!(ptr::addr_of!(_bss_start) as usize),
        aligh_4k!(ptr::addr_of!(_bss_end) as usize),
        vm::EntryBits::ReadWrite.val(),
    );

    ident_range_map(
        pageroot,
        aligl_4k!(ptr::addr_of!(_stack_end) as usize),
        aligh_4k!(ptr::addr_of!(_stack_start) as usize),
        vm::EntryBits::ReadWrite.val(),
    );

    ident_range_map(
        pageroot,
        meta_begin,
        meta_end,
        vm::EntryBits::ReadWrite.val(),
    );

    //uart mmio area
    ident_range_map(
        pageroot,
        aligl_4k!(ptr::addr_of!(_virtio_start) as usize),
        aligh_4k!(ptr::addr_of!(_virtio_end) as usize),
        vm::EntryBits::ReadWrite.val(),
    );

    //qemu mmio memory mapping according to qemu/hw/riscv/virt.c

    //CLIENT
    ident_range_map(
        pageroot,
        0x0200_0000,
        0x0200_ffff,
        vm::EntryBits::ReadWrite.val(),
    );

    //PLIC
    unsafe {
        ident_range_map(
            pageroot,
            PLIC.base,
            PLIC.base + (plic_ctx::max_ctx() + 2) * 0x1000,
            vm::EntryBits::ReadWrite.val(),
        );

        ident_range_map(
            pageroot,
            PLIC.thres_base,
            PLIC.thres_base + plic_ctx::max_ctx() * 0x1000,
            vm::EntryBits::ReadWrite.val(),
        );
    }

    let paddr = 0x1000_0000 as usize;
    let vaddr = virt2phys(&pageroot, paddr)?.unwrap_or(0);

    println!("VM Walker test: Paddr: {:#x} -> Vaddr: {:#x}", paddr, vaddr);

    /*
     * Memory allocation for trap stack
     */
    unsafe {
        for cpu_cnt in 0..cpu::MAX_HARTS {
            KERNEL_TRAP_FRAME[cpu_cnt].cpuid = cpu_cnt;

            KERNEL_TRAP_FRAME[cpu_cnt].trap_stack =
                kmalloc_page(zone_type::ZONE_NORMAL, 2)?.add(page::PAGE_SIZE * 2);

            ident_range_map(
                pageroot,
                KERNEL_TRAP_FRAME[cpu_cnt]
                    .trap_stack
                    .sub(2 * page::PAGE_SIZE) as usize,
                KERNEL_TRAP_FRAME[cpu_cnt].trap_stack as usize,
                vm::EntryBits::ReadWrite.val(),
            );

            let trapstack_paddr = KERNEL_TRAP_FRAME[cpu_cnt].trap_stack as usize - 1;
            let trapstack_vaddr = virt2phys(&pageroot, trapstack_paddr)?.unwrap_or(0);

            println!(
                "CPU#{} TrapStack: (vaddr){:#x} -> (paddr){:#x}",
                cpu_cnt, trapstack_paddr, trapstack_vaddr
            );

            let trapfram_paddr = ptr::addr_of_mut!(KERNEL_TRAP_FRAME[cpu_cnt]) as usize;
            let trapfram_vaddr = virt2phys(&pageroot, trapfram_paddr)?.unwrap_or(0);

            println!(
                "CPU#{} TrapFrame: (vaddr){:#x} -> (paddr){:#x}",
                cpu_cnt, trapfram_paddr, trapfram_vaddr
            );
        }
    }

    /*
     * Set up satp register to provide paging mode and PPN of
     * root page table
     */
    cpu::satp_write(SATP_mode::Sv39, 0, pageroot_ptr as usize);

    kmem::set_ksatp(cpu::satp_read());
    /*
     * Set up arrival address of S-mode entry
     */
    cpu::mepc_write(eh_func_kmain as usize);

    /*
     * We only delegate ext interrupt and all exception to S-mode
     *
     * timer needs to be handled in M-mode since we need access to CLINT, as well as sw interrupt
     */

    unsafe {
        CLINT.set_mtimecmp(current_cpu, u64::MAX);

        mie::set_msoft();

        mie::set_mtimer();

        mie::set_mext();
        mie::set_sext();

        // mideleg::set_sext();
        sie::set_sext();
        sstatus::set_spie();

        /* TODO:
         * Get rid this ugly written code and replace with fancy vector
         */
        EXTINT_SRCS[10].set_name(extint_name::UART0);
        EXTINT_SRCS[10].set_id(10);
        PLIC.set_prio(&EXTINT_SRCS[10], 5)?;
        PLIC.enable(plic_ctx::CORE0_M, &EXTINT_SRCS[10])?;
        PLIC.enable(plic_ctx::CORE1_M, &EXTINT_SRCS[10])?;
        PLIC.enable(plic_ctx::CORE2_M, &EXTINT_SRCS[10])?;
        PLIC.enable(plic_ctx::CORE3_M, &EXTINT_SRCS[10])?;
        mstatus::set_mpp(mstatus::MPP::Supervisor);
    }

    cpu::sfence_vma();

    /*
     * Unlock other cores from early spin lock
     */
    unsafe {
        let early_boot: *mut u64 = ptr::addr_of_mut!(cpu_early_block);
        early_boot.write_volatile(0xffff_ffff);
    }

    Ok(0)
}

fn kmain(current_cpu: usize) -> Result<(), KError> {
    println!("CPU#{} Switched to S mode", current_cpu);

    unsafe {
        asm!("ebreak");

        println!("CPU{} Back from trap\n", current_cpu);
        CLINT.set_mtimecmp(current_cpu, CLINT.read_mtime() + 0x500_000);
    }

    let k = alloc::vec![1, 2, 3, 4, 5];
    for i in k.iter() {
        println!("{}", i);
    }

    println!("---------->>Start Process<<----------");
    unsafe {
        TASK_POOL.init(cpu::MAX_HARTS);

        let mut pcb_khello: task_struct = task_struct::new();
        let sched_cpu = which_cpu();

        pcb_khello.init();

        TASK_POOL.append_task(&pcb_khello, sched_cpu)?;
        TASK_POOL.sched(sched_cpu)?;
    }

    loop {
        println!("CPU#{} kmain keep running...", current_cpu);
        let _ = cpu::busy_delay(1);
        unsafe {
            asm!("nop");
        }
    }

    Ok(())
}

pub mod allocator;
pub mod clint;
pub mod cpu;
pub mod ecall;
pub mod error;
pub mod kmem;
pub mod lock;
pub mod nobsp_kfunc;
pub mod page;
pub mod plic;
pub mod proc;
pub mod trap;
pub mod uart;
pub mod vm;
pub mod zone;
