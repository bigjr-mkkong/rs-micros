
pub const CLINT_BASE:usize = 0x200_0000;

pub struct clint_controller{
    base_addr: usize
}

impl clint_controller{
    pub const fn new(base: usize) -> Self{
        clint_controller{
            base_addr: base
        }
    }

    pub fn set_mtimecmp(&self, hartid: usize, new_val: u64){
        let mtimecmp_base = (self.base_addr + 0x4000) as *mut u64;
        unsafe{
            mtimecmp_base.add(hartid).write_volatile(new_val);
        }
    }

    pub fn read_mtime(&self) -> u64{
        let mtime = (self.base_addr + 0xbff8) as *const u64;
        unsafe{
            mtime.read_volatile()
        }

    }
}
