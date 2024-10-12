#![no_std]

#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(variant_count)]

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
use spin::Mutex;
use riscv::register::{mstatus, mideleg, medeleg, mie, sie};

use error::{KError, KErrorType};
use zone::{zone_type, kmalloc_page, kfree_page};
use vm::{ident_range_map, virt2phys};
use cpu::{SATP_mode, TrapFrame, which_cpu};
use crate::lock::spin_mutex;
use crate::lock::{M_lock, S_lock, ALL_lock};
use plic::{plic_controller, plic_ctx, extint_map};
use nobsp_kfunc::kinit as nobsp_kinit;
use nobsp_kfunc::kmain as nobsp_kmain;
use clint::clint_controller;
use ecall::{ecall_args, S2Mop};

#[macro_export]
macro_rules! print
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        let _ = write!(SYS_UART.lock(), $($args)+);
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
        println!("line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message());
    }else{
        println!("PanicInfo not available yet");
    }

    abort();
}

#[no_mangle]
extern "C"
fn abort() -> ! {
    loop{
        unsafe{
            asm!("nop");
        }
    }
}


#[no_mangle]
extern "C"
fn eh_func_kinit() -> usize{
    let cpuid = cpu::mhartid_read();
    unsafe{
        cpu::sscratch_write((&mut KERNEL_TRAP_FRAME[0] as *mut TrapFrame) as usize);
        cpu::mscratch_write(cpu::sscratch_read());
        KERNEL_TRAP_FRAME[cpuid].cpuid = cpuid;
    }
    cpu::set_cpu_mode(cpu::Mode::Machine, cpuid);
    let init_return = kinit();
    if let Err(er_code) = init_return{
        println!("{}", er_code);
        println!("kinit() Failed on CPU#{}, System halting now...", cpuid);
        loop{
            unsafe{
                asm!("nop");
            }
        }
    }else{
        init_return.unwrap_or_default()
    }
}

#[no_mangle]
extern "C"
fn eh_func_kmain(){
    let cpuid = which_cpu();
    cpu::set_cpu_mode(cpu::Mode::Supervisor, cpuid);
    let main_return = kmain();
    if let Err(er_code) = main_return{
        println!("{}", er_code);
        println!("kmain() Failed, System halting now...");
        loop{
            unsafe{
                asm!("nop");
            }
        }
    }
}

#[no_mangle]
extern "C"
fn eh_func_kinit_nobsp() -> usize{
    let cpuid = cpu::mhartid_read();
    let init_return = nobsp_kinit();
    if let Err(er_code) = init_return{
        println!("{}", er_code);
        println!("nobsp_kinit() Failed at CPU#{}, System halting now...", cpuid);
        loop{
            unsafe{
                asm!("nop");
            }
        }
    }else{
        init_return.unwrap_or_default()
    }
}

#[no_mangle]
pub extern "C"
fn eh_func_nobsp_kmain(){
    let main_return = nobsp_kmain();
    if let Err(er_code) = main_return{
        println!("{}", er_code);
        println!("kmain() Failed, System halting now...");
        loop{
            unsafe{
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
pub const ZONE_DEFVAL:spin_mutex<zone::mem_zone, ALL_lock> =
    spin_mutex::<zone::mem_zone, ALL_lock>::new(zone::mem_zone::new());

pub static SYS_ZONES: [spin_mutex<zone::mem_zone, ALL_lock>; 3] = [
    ZONE_DEFVAL; zone_type::type_cnt()
];

/*
 * TODO:
 * We need to work around uart for lock
 * We cannot specify uart as ALL_lock
 * I think we can have two uart, one for M mode and one for S mode
 */
pub static SYS_UART: spin_mutex<uart::Uart, S_lock> =
    spin_mutex::<uart::Uart, S_lock>::new(uart::Uart::new(0x1000_0000));

pub static mut KERNEL_TRAP_FRAME: [TrapFrame; 8] = [TrapFrame::new(); 8];
pub static mut PLIC: plic_controller = plic_controller::new(plic::PLIC_BASE);
pub static mut CLINT: clint_controller = clint_controller::new(clint::CLINT_BASE);
pub static mut SECALL_FRAME: [ecall_args; cpu::MAX_HARTS] = [ecall_args::new(); cpu::MAX_HARTS ];

fn kinit() -> Result<usize, KError> {
    SYS_UART.lock().init();

    println!("\nHello world");

    let current_cpu = cpu::mhartid_read();
    println!("Initializer running on CPU#{}", current_cpu);

    /*
     * Setting up new zone
     */
    SYS_ZONES[zone_type::ZONE_NORMAL.val()].lock().init(
            ptr::addr_of!(_heap_start),
            ptr::addr_of!(_heap_end),
            zone_type::ZONE_NORMAL,
        zone::AllocatorSelector::NaiveAllocator)?;

    SYS_ZONES[zone_type::ZONE_UNDEF.val()].lock().init(
        0 as *const u8,
        0 as *const u8,
        zone_type::ZONE_UNDEF,
        zone::AllocatorSelector::EmptyAllocator)?;

    kmem::init()?;

    let pageroot_ptr = kmem::get_page_table();
    let mut pageroot = unsafe{pageroot_ptr.as_mut().unwrap()};

    let kheap_begin = kmem::get_kheap_start();
    let kheap_pgcnt = kmem::get_kheap_pgcnt();

    ident_range_map(pageroot,
            aligl_4k!(ptr::addr_of!(_text_start) as usize),
            aligh_4k!(ptr::addr_of!(_text_end) as usize),
            vm::EntryBits::ReadExecute.val());

    // let usz_heap_start = ptr::addr_of!(_heap_start) as usize;
    // let usz_heap_end = usz_heap_start + SYS_ZONES[zone_type::ZONE_NORMAL.val()].lock().get_size()?;
    // ident_range_map(pageroot,
    //         usz_heap_start,
    //         usz_heap_end,
    //         vm::EntryBits::ReadWrite.val());

    ident_range_map(pageroot, 
            kheap_begin as usize,
            kheap_begin as usize + page::PAGE_SIZE * kheap_pgcnt,
            vm::EntryBits::ReadWrite.val());

    ident_range_map(pageroot,
            aligl_4k!(ptr::addr_of!(_rodata_start) as usize),
            aligh_4k!(ptr::addr_of!(_rodata_end) as usize),
            vm::EntryBits::ReadExecute.val());

    ident_range_map(pageroot,
            aligl_4k!(ptr::addr_of!(_data_start) as usize),
            aligh_4k!(ptr::addr_of!(_data_end) as usize),
            vm::EntryBits::ReadWrite.val());

    ident_range_map(pageroot,
            aligl_4k!(ptr::addr_of!(_bss_start) as usize),
            aligh_4k!(ptr::addr_of!(_bss_end) as usize),
            vm::EntryBits::ReadWrite.val());

    ident_range_map(pageroot,
            aligl_4k!(ptr::addr_of!(_stack_end) as usize),
            aligh_4k!(ptr::addr_of!(_stack_start) as usize),
            vm::EntryBits::ReadWrite.val());


//uart mmio area
    ident_range_map(pageroot,
            aligl_4k!(ptr::addr_of!(_virtio_start) as usize),
            aligh_4k!(ptr::addr_of!(_virtio_end) as usize),
            vm::EntryBits::ReadWrite.val());

//qemu mmio memory mapping according to qemu/hw/riscv/virt.c
    
    //CLIENT
    ident_range_map(pageroot,
            0x0200_0000,
            0x0200_ffff,
            vm::EntryBits::ReadWrite.val());


    //PLIC
    unsafe{
        ident_range_map(pageroot,
                PLIC.base,
                PLIC.base + (plic_ctx::max_ctx() + 2) * 0x1000,
                vm::EntryBits::ReadWrite.val());

        ident_range_map(pageroot,
                PLIC.thres_base,
                PLIC.thres_base + plic_ctx::max_ctx() * 0x1000,
                vm::EntryBits::ReadWrite.val());
    }
    

    let paddr = 0x1000_0000 as usize;
    let vaddr = virt2phys(&pageroot, paddr)?.unwrap_or(0);

    println!("VM Walker test: Paddr: {:#x} -> Vaddr: {:#x}", paddr, vaddr);
    
    /*
     * Memory allocation for trap stack
     */
    unsafe{

        KERNEL_TRAP_FRAME[current_cpu].trap_stack = 
                kmalloc_page(zone_type::ZONE_NORMAL, 2)?.add(page::PAGE_SIZE);

        ident_range_map(pageroot, 
                KERNEL_TRAP_FRAME[current_cpu].trap_stack.sub(2 * page::PAGE_SIZE) as usize,
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

    /*
     * Set up satp register to provide paging mode and PPN of 
     * root page table
     */
    cpu::satp_write(SATP_mode::Sv39, 0, pageroot_ptr as usize);

    /*
     * Set up arrival address of S-mode entry
     */
    cpu::mepc_write(eh_func_kmain as usize);


    /*
     * We only delegate ext interrupt and all exception to S-mode
     *
     * timer needs to be handled in M-mode since we need access to CLINT, as well as sw interrupt
     */

    unsafe{
        CLINT.set_mtimecmp(current_cpu, u64::MAX);
        
        mideleg::set_sext();

        mie::set_mext();
        mie::set_msoft();
        mie::set_mtimer();

        mie::set_sext();

        mstatus::set_mpp(mstatus::MPP::Supervisor);
    }

    cpu::sfence_vma();

    /*
     * Unlock other cores from early spin lock
     */
    unsafe{
        let early_boot: *mut u64 = ptr::addr_of_mut!(cpu_early_block);
        early_boot.write_volatile(0xffff_ffff);
    }

    Ok(0)
}

fn kmain() -> Result<(), KError> {
    let current_cpu = which_cpu();;
    println!("CPU#{} Switched to S mode", current_cpu);
    
    unsafe{
        asm!("ebreak");

        println!("Back from trap\n");
        CLINT.set_mtimecmp(current_cpu, CLINT.read_mtime() + 0x500_000);
    }


    loop{
        unsafe{
            asm!("nop");
        }
    }
}


pub mod uart;
pub mod zone;
pub mod error;
pub mod page;
pub mod vm;
pub mod kmem;
pub mod trap;
pub mod cpu;
pub mod nobsp_kfunc;
pub mod plic;
pub mod lock;
pub mod clint;
pub mod ecall;
