use core::mem;
use core::array;

use crate::error::{KError, KErrorType};

#[derive(Clone, Copy, PartialEq)]
pub enum zone_type{
    ZONE_UNDEF,
    ZONE_NORMAL,
    ZONE_VIRTIO
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

pub trait page_allocator{
    fn allocator_init(&mut self, zone_start: *mut u8, zone_end: *mut u8, zone_size: usize) -> Result<(), KError>;
    fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError>;
    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError>;
}

#[derive(Clone, Copy)]
pub struct mem_zone<A: page_allocator>{
    begin_addr: *mut u8,
    end_addr: *mut u8,
    zone_size: usize,
    types: zone_type,
    pg_allocator: A
}

impl<A: page_allocator + Default> mem_zone<A>{
    pub fn new() -> Self{
        mem_zone{
            begin_addr: 0 as *mut u8,
            end_addr: 0 as *mut u8,
            zone_size: 0,
            types: zone_type::ZONE_UNDEF,
            pg_allocator: A::default()
        }
    }

    pub fn init(&mut self, _start: *mut u8, _end: *mut u8, _type: zone_type, allocator: A) -> Result<(), KError>{
        self.begin_addr = _start;
        self.end_addr = _end;
        self.zone_size = (unsafe{_end.offset_from(_start)}) as usize;
        self.types = _type;
        self.pg_allocator = allocator;

        self.pg_allocator.allocator_init(self.begin_addr, self.end_addr, self.zone_size)?;

        Ok(())
    }

    pub fn print_all(&self) {
        println!("[ZONE INFO] Begin: {:#x} -> End: {:#x}  Size: {:#x}  Type: {:#?}",
            self.begin_addr as usize,
            self.end_addr as usize,
            self.zone_size,
            self.types.as_str());
    }

    pub fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError> {
        self.pg_allocator.alloc_pages(pg_cnt)
    }

    pub fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError> {
        self.pg_allocator.free_pages(addr)
    }
}

pub struct system_zones<A: page_allocator>{
    next_zone: usize,
    zones: [mem_zone<A>; ZONE_CNT]
}

impl<A: page_allocator + core::default::Default + core::marker::Copy> system_zones<A>{
    pub fn new() -> Self{
        system_zones{
            next_zone: 0,
            zones: [mem_zone::new(); ZONE_CNT]
        }
    }

    pub fn add_newzone(&mut self, zone_begin: *mut u8, zone_end:*mut u8, ztype:zone_type, alloc: A) -> Result<(), KError> {
        self.zones[self.next_zone].init(zone_begin, zone_end, ztype, alloc)?;
        self.next_zone += 1;
        Ok(())
    }

    pub fn print_all(&self) {
        for i in 0..self.next_zone{
            self.zones[i].print_all();
        }
    }

    pub fn get_from_type(&mut self, target_type: zone_type) -> Option<&mut mem_zone<A>>{
        if let zone_type::ZONE_UNDEF = target_type {
            return None;
        }

        for z in self.zones.iter_mut(){
            if target_type == z.types{
                return Some(z);
            }
        }

        None
    }
}
