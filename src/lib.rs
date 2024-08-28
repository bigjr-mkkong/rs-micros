#![no_std]
#![feature(panic_info_message)]

#![allow(unused)]
#![allow(non_camel_case_types)]

extern "C" {
    static mut HEAP_START: u8;
    static mut HEAP_END: u8;
    static mut VIRTIO_START: u8;
    static mut VIRTIO_END: u8;
}

use core::arch::asm;
use core::ptr;
use error::{KError, KErrorType};
use zone::{zone_type, kmalloc_page, kfree_page};
use spin::RwLock;

#[macro_export]
macro_rules! print
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        let _ = write!(crate::uart::Uart::new(0x10000000), $($args)+);
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
    print!("Aborting...");
    if let Some(p) = info.location() {
        println!("line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message().unwrap());
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
fn eh_func(){
    let init_return = kmain();
    if let Err(er_code) = init_return{
        println!("{}", er_code);
        println!("SYSTEM HALTING NOW");
        loop{
            unsafe{
                asm!("nop");
            }
        }
    }

}

static sys_zone: [spin::Mutex<zone::mem_zone>; 3] = [
    spin::Mutex::new(zone::mem_zone::new()),
    spin::Mutex::new(zone::mem_zone::new()),
    spin::Mutex::new(zone::mem_zone::new()),
];

fn kmain() -> Result<(), KError> {
    let mut uart = uart::Uart::new(0x1000_0000);
    uart.init();

    println!("\nHello world");

    // let mut sys_zones = zone::system_zones::new();

    // let allocator = page::naive_allocator::default();
    // let null_allocator = page::empty_allocator::new();

    let zone_start;
    let zone_end;
    /*
     * Setting up new zone
     */
    unsafe{
        zone_start = ptr::addr_of_mut!(HEAP_START) as *mut u8;
        zone_end = ptr::addr_of_mut!(HEAP_END) as *mut u8;
    }
    sys_zone[zone_type::ZONE_NORMAL.val()].lock().init(zone_start, zone_end, zone_type::ZONE_NORMAL,
        zone::AllocatorSelector::NaiveAllocator)?;
    sys_zone[zone_type::ZONE_UNDEF.val()].lock().init(0 as *mut u8, 0 as *mut u8, zone_type::ZONE_UNDEF,
        zone::AllocatorSelector::EmptyAllocator)?;

    let pg = kmalloc_page(zone_type::ZONE_NORMAL, 1)?;
    println!("New page:{:#x}", pg as usize);
    kfree_page(zone_type::ZONE_NORMAL, pg)?;

    loop{
        if let Some(c) = uart.get() {
            println!("{}", c as char);
        }
        unsafe{
            asm!("nop");
        }
    }
    Ok(())
}

pub mod uart;
pub mod zone;
pub mod error;
pub mod page;
