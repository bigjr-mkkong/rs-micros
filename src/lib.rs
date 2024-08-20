#![no_std]
#![feature(panic_info_message, asm)]

use core::arch::asm;

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
    let mut UART = uart::Uart::new(0x10000000);
    UART.init();

    println!("\nHello world");

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
