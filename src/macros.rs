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
        use crate::Mprint;
        Mprint!("\r\n")
    });

    ($fmt:expr) => ({
        use crate::Mprint;
        Mprint!(concat!($fmt, "\r\n"))
    });

    ($fmt:expr, $($args:tt)+) => ({
        use crate::Mprint;
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
        use crate::Sprint;
        Sprint!("\r\n")
    });

    ($fmt:expr) => ({
        use crate::Sprint;
        Sprint!(concat!($fmt, "\r\n"))
    });

    ($fmt:expr, $($args:tt)+) => ({
        use crate::Sprint;
        Sprint!(concat!($fmt, "\r\n"), $($args)+)
    });

}

#[macro_export]
macro_rules! GETRSETR {
    ($name:ident, $type:ty) => {
        $crate::paste::paste! {
            pub fn [<get_ $name>](&self) -> $type {
                self.$name
            }

            pub fn [<set_ $name>](&mut self, new_val: $type) {
                self.$name = new_val;
            }
        }
    };
}
