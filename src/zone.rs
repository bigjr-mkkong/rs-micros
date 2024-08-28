use core::mem;
use core::array;

use crate::sys_uart;
use crate::error::{KError, KErrorType};
use crate::new_kerror;

#[derive(Clone, Copy, PartialEq)]
pub enum zone_type{
    ZONE_UNDEF = 0,
    ZONE_NORMAL = 1,
    ZONE_VIRTIO = 2
}

impl zone_type{
    pub fn val(&self) -> usize{
        *self as usize
    }
}

const ZONE_CNT:usize = 10;

impl zone_type{
    pub fn as_str(&self) -> &str{
        match self{
            zone_type::ZONE_NORMAL => {
                "ZONE_NORMAL"
            },
            zone_type::ZONE_VIRTIO => {
                "ZONE_VIRTIO"
            },
            zone_type::ZONE_UNDEF => {
                "ZONE_UNDEF"
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct mem_zone{
    pub begin_addr: usize,
    pub end_addr: usize,
    pub zone_size: usize,
    pub types: zone_type,
}

impl mem_zone{
    pub const fn new() -> Self{
        mem_zone{
            begin_addr: 0,
            end_addr: 0,
            zone_size: 0,
            types: zone_type::ZONE_UNDEF,
        }
    }

    pub fn init(&mut self, _start: *mut u8, _end: *mut u8, _type: zone_type) -> Result<(), KError>{
        self.begin_addr = _start as usize;
        self.end_addr = _end as usize;
        self.zone_size = (unsafe{_end.offset_from(_start)}) as usize;
        self.types = _type;

        Ok(())
    }

    pub fn print_all(&self) {
        println!("[ZONE INFO] Begin: {:#x} -> End: {:#x}  Size: {:#x}  Type: {:#?}",
            self.begin_addr as usize,
            self.end_addr as usize,
            self.zone_size,
            self.types.as_str());
    }

}

pub struct system_zones{
    zone_cnt: usize,
    zones: [mem_zone; ZONE_CNT]
}

impl system_zones{
    pub const fn new() -> Self{
        system_zones{
            zone_cnt: 0,
            zones: [mem_zone::new(); ZONE_CNT]
        }
    }

    pub fn add_newzone(&mut self, zone_begin: *mut u8, zone_end:*mut u8, ztype:zone_type) -> Result<(), KError> {
        self.zones[ztype.val()].init(zone_begin, zone_end, ztype)?;
        self.zone_cnt += 1;
        Ok(())
    }

    pub fn print_all(&self) {
        for z in self.zones{
            if let zone_type::ZONE_UNDEF = z.types{
                continue;
            }
            
            z.print_all();
        }
    }

    pub fn get_from_type(&self, target_type: zone_type) -> Option<&mem_zone>{
        if let zone_type::ZONE_UNDEF = self.zones[target_type.val()].types {
            None
        }else{
            Some(&self.zones[target_type.val()])
        }
    }
}
