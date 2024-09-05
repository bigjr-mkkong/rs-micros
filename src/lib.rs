#![no_std]

#![allow(unused)]
#![allow(non_camel_case_types)]

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
}

use core::arch::asm;
use core::mem::size_of;
use core::ptr;
use error::{KError, KErrorType};
use zone::{zone_type, kmalloc_page, kfree_page};
use spin::{Mutex, RwLock};
use vm::{ident_range_map, virt2phys};
use cpu::{SATP_mode, TrapFrame};

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
fn eh_func_kinit_nobsp() -> usize{

    /*
     * TODO
     * Implement trapstack memory initializatin for non-bsp core
     */
    loop{
        unsafe{
            asm!("nop");
        }
    }
}

#[no_mangle]
extern "C"
fn eh_func_kmain(){
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

//   ____ _     ___  ____    _    _      __     ___    ____  ____  
//  / ___| |   / _ \| __ )  / \  | |     \ \   / / \  |  _ \/ ___| 
// | |  _| |  | | | |  _ \ / _ \ | |      \ \ / / _ \ | |_) \___ \ 
// | |_| | |__| |_| | |_) / ___ \| |___    \ V / ___ \|  _ < ___) |
//  \____|_____\___/|____/_/   \_\_____|    \_/_/   \_\_| \_\____/ 
pub const zone_defval:Mutex<zone::mem_zone> = spin::Mutex::new(zone::mem_zone::new());
pub static SYS_ZONES: [spin::Mutex<zone::mem_zone>; 3] = [
    zone_defval; zone_type::type_cnt()
];
pub static SYS_UART: Mutex<uart::Uart> = Mutex::new(uart::Uart::new(0x1000_0000));
pub static mut KERNEL_TRAP_FRAME: [TrapFrame; 8] = [TrapFrame::new(); 8];


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
    ident_range_map(pageroot,
            0x0c00_0000,
            0x0c00_2000,
            vm::EntryBits::ReadWrite.val());


    let paddr = 0x1000_0000 as usize;
    let vaddr = virt2phys(&pageroot, paddr)?.unwrap_or(0);

    println!("VM Walker test: Paddr: {:#x} -> Vaddr: {:#x}", paddr, vaddr);
    
    /*
     * TODO:
     * We need a well-srtuctured rust rv64gc Machine Abstraction for those csr(s)
     */
    
    /*
     * Memory allocation for trap stack
     */
    unsafe{
        cpu::mscratch_write((&mut KERNEL_TRAP_FRAME[0] as *mut TrapFrame) as usize);
        cpu::sscratch_write(cpu::mscratch_read());

        KERNEL_TRAP_FRAME[current_cpu].trap_stack = 
                kmalloc_page(zone_type::ZONE_NORMAL, 1)?.add(page::PAGE_SIZE);

        ident_range_map(pageroot, 
                KERNEL_TRAP_FRAME[current_cpu].trap_stack.sub(page::PAGE_SIZE) as usize,
                KERNEL_TRAP_FRAME[current_cpu].trap_stack as usize,
                vm::EntryBits::ReadWrite.val());

        // ident_range_map(pageroot,
        //         cpu::mscratch_read(),
        //         cpu::mscratch_read() + (current_cpu+1) * size_of::<TrapFrame>(),
        //         vm::EntryBits::ReadWrite.val());


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

    unsafe{
        asm!("ebreak");
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
     * enable S-mode + MPIE + SPIE
     */
    cpu::mstatus_write((1 << 11) | (1 << 7) | (1 << 5) as usize);

    /*
     * Now we only consider sw interrupt, timer and external
     * interrupt will be enabled in future
     */
    cpu::mie_write((1 << 3) as usize);
    
    cpu::sfence_vma();

    Ok(0)
}

fn kmain() -> Result<(), KError> {
    println!("Switched to S mode");

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
pub mod uart;
pub mod zone;
pub mod error;
pub mod page;
pub mod vm;
pub mod kmem;
pub mod trap;
pub mod cpu;
