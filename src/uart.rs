use core::convert::TryInto;
use core::fmt::{Error, Write};

pub struct Uart{
    base_address: usize,
}

impl Write for Uart{
    fn write_str(&mut self, out: &str) -> Result<(), core::fmt::Error>{
        for c in out.bytes(){
            self.put(c);
        }

        Ok(())
    }
}

impl Uart{
    pub fn new(base_address: usize) -> Self{
        Uart{
            base_address
        }
    }

    pub fn init(&mut self) {
        let ptr = self.base_address as *mut u8;
        unsafe{
            ptr.add(3).write_volatile((1<<0) | (1 << 1));
            ptr.add(2).write_volatile(1<<0);
            ptr.add(1).write_volatile(1<<0);
        
            let div:u16 = 592;
            let div_lsb:u8 = (div & 0xff).try_into().unwrap();
            let div_msb:u8 = (div >> 8).try_into().unwrap();
            
            let lcr = ptr.add(3).read_volatile();
            ptr.add(3).write_volatile(lcr);
        }
    }

    pub fn put(&mut self, ch: u8){
        let ptr = self.base_address as *mut u8;
        unsafe{
            ptr.add(0).write_volatile(ch);
        }
    }

    pub fn get(&mut self) -> Option<u8>{
        let ptr = self.base_address as *mut u8;
        unsafe{
            if ptr.add(5).read_volatile() & 1 == 0 {
                None
            }else{
                Some(ptr.add(0).read_volatile())
            }
        }
    }

}

