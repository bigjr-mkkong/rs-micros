#![no_std]
#![feature(panic_info_message, asm)]

extern "C" {
    static mut HEAP_START: u8;
    static mut HEAP_END: u8;
    static mut VIRTIO_START: u8;
    static mut VIRTIO_END: u8;
}

use core::arch::asm;
use core::ptr;

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
        sys_zones.add_newzone(ptr::addr_of_mut!(HEAP_START)as *mut u8,
            ptr::addr_of_mut!(HEAP_END) as *mut u8, zone::zone_type::ZONE_NORMAL);

        sys_zones.add_newzone(ptr::addr_of_mut!(VIRTIO_START)as *mut u8,
            ptr::addr_of_mut!(VIRTIO_END) as *mut u8, zone::zone_type::ZONE_VIRTIO);
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
