#![no_std]
#![feature(panic_info_message, asm)]

extern "C" {
    static HEAP_START: u8;
    static HEAP_SIZE: u8;
}

use core::arch::asm;
use core::mem::MaybeUninit;

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
fn kmain(){
    let mut UART = uart::Uart::new(0x1000_0000);
    UART.init();

    println!("\nHello world");

    let mut sys_zones = zone::system_zones::new();

    unsafe{
        let begin_ref: *const u8 = &HEAP_START as *const u8;
        let size_ref: *const u8 = &HEAP_SIZE as *const u8;
        sys_zones.add_newzone(begin_ref as usize, size_ref as usize, zone::zone_type::ZONE_NORMAL);
        sys_zones.print_all();
    }

    loop{
        if let Some(c) = UART.get() {
            println!("{}", c as char);
        }
        unsafe{
            asm!("nop");
        }
    }
}


pub mod uart;
pub mod zone;
