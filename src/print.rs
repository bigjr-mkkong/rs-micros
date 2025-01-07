use crate::{M_UART, S_UART};
use core::fmt::Write;

#[macro_export]
macro_rules! Mprint
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        use crate::cpu;
        let _ = write!(M_UART.lock(), $($args)+);
    });
}

#[macro_export]
macro_rules! Mprintln
{
    () => ({
        Mprint!("\r\n")
    });

    ($fmt:expr) => ({
        Mprint!(concat!($fmt, "\r\n"))
    });

    ($fmt:expr, $($args:tt)+) => ({
        Mprint!(concat!($fmt, "\r\n"), $($args)+)
    });

}

#[macro_export]
macro_rules! Sprint
{
    ($($args:tt)+) => ({
        use core::fmt::Write;
        use crate::cpu;
        let _ = write!(S_UART.lock(), $($args)+);
    });
}

#[macro_export]
macro_rules! Sprintln
{
    () => ({
        Sprint!("\r\n")
    });

    ($fmt:expr) => ({
        Sprint!(concat!($fmt, "\r\n"))
    });

    ($fmt:expr, $($args:tt)+) => ({
        Sprint!(concat!($fmt, "\r\n"), $($args)+)
    });

}
