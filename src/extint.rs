/*
 * We are using is SiFive PLIC, here is the spec:
 *https://sifive.cdn.prismic.io/sifive%2F834354f0-08e6-423c-bf1f-0cb58ef14061_fu540-c000-v1.0.pdf#%5B%7B%22num%22%3A164%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C0%2C630%2C0%5D
 */

use crate::SYS_UART;
use crate::{KError, KErrorType};
use crate::new_kerror;
use crate::cpu::{which_cpu, MAX_HARTS};
use plic::InterruptSource;
use core::num::NonZeroU32;

pub const PLIC_BASE: usize = 0x0c00_0000;
pub const NUM_INTERRUPTS: usize = 20;
pub const PLIC_RNG1_BEGIN:usize = 0x0c00_0000;
pub const PLIC_RNG1_END:usize = 0x0c00_2000;

pub const PLIC_RNG2_BEGIN: usize = 0x0c20_0000;
pub const PLIC_RNG2_END: usize = 0x1000_0000;

enum intmap{
    UART0_SENDRECV,
    UART0_LINESTAT,
    TIMER,
    GPIO,
    VIRTIO_NET,
    VIRTIO_BLK,
}

impl InterruptSource for intmap{
    fn id(self) -> NonZeroU32{
        let val:u32 = match self{
            intmap::UART0_SENDRECV =>   0,
            intmap::UART0_LINESTAT =>   1,
            intmap::TIMER =>            2,
            intmap::GPIO =>             3,
            intmap::VIRTIO_NET =>       4,
            intmap::VIRTIO_BLK =>       5,
        };

        NonZeroU32::new(val).unwrap()
    }
}
