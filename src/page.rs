use core::ptr;
use core::mem;
use core::array;

use crate::zone;
use crate::error::{KError, KErrorType};

pub const PAGE_SIZE:usize = 4096;

macro_rules! aligh_4k{
    ($n:expr) => {
        ($n as usize + 4096 - 1) & !(4096 - 1)
    };
}

macro_rules! aligl_4k{
    ($n:expr) => {
        ($n as usize) & !(4096 - 1)
    };
}

enum page_flag{
    PF_FREE = (1 << 0),
    PF_TAKEN = (1 << 1)
}

struct PageMark{
    flags: u8
}

struct PageRec{
    begin: *const u8,
    len: usize,
    inuse: bool
}


/*
 * naive_allocator needs at least 4 Pages memory to works
 */
#[derive(Clone, Copy)]
pub struct naive_allocator{
    tot_page: usize,
    zone_begin: *mut u8,
    zone_end: *mut u8,
    map_begin: *mut u8,
    map_size: usize,
    mem_begin: *const u8,
    mem_end: *const u8,
    rec_begin: *mut u8,
    rec_size: usize
}

impl Default for naive_allocator{
    fn default() -> Self{
        naive_allocator{
            tot_page: 0,
            zone_begin: ptr::null_mut(),
            zone_end: ptr::null_mut(),
            map_begin: ptr::null_mut(),
            map_size: 0,
            rec_begin: ptr::null_mut(),
            rec_size: 0,
            mem_begin: ptr::null(),
            mem_end: ptr::null(),
        }
    }
}

impl zone::page_allocator for naive_allocator{
    fn allocator_init(&mut self, zone_start: *mut u8, zone_end: *mut u8, zone_size: usize) -> Result<(), KError>{

        //Pretty wild, but lets keep this since this is a NAIVE allocator
        if zone_size < 3 * PAGE_SIZE { 
            return Err(KError::new(KErrorType::ENOMEM));
        }

        self.zone_begin = zone_start;
        self.zone_end = zone_end;

        self.mem_end = aligl_4k!(zone_end) as *const u8;
        self.map_begin = aligh_4k!(zone_start) as *mut u8;

        self.tot_page = unsafe{self.mem_end.offset_from(self.map_begin) as usize / PAGE_SIZE};
        self.map_size = aligh_4k!(self.tot_page * mem::size_of::<PageMark>());
        self.rec_begin = unsafe{self.map_begin.add(self.map_size)};
        self.rec_size = aligh_4k!(self.tot_page * mem::size_of::<PageRec>());
        self.mem_begin = unsafe{self.rec_begin.add(self.rec_size) as *const u8};
        self.tot_page = unsafe{self.mem_end.offset_from(self.mem_begin) as usize / PAGE_SIZE};

        self.print_info();

        Ok(())
    }

    fn alloc_pages(&mut self, pg_cnt: usize) -> Option<*mut u8> {
        println!("Start allocate {} page(s)", pg_cnt);
        None
    }

    fn free_pages(&mut self, addr: *mut u8) {
        println!("Start reclaiming...");
    }
}

impl naive_allocator{
    fn print_info(&self) {
        println!("------------Allocator Info------------");
        println!("Mapping Begin: {:#x} -- Size: {:#x}",
                self.map_begin as usize, self.map_size);
        println!("Record Begin: {:#x} -- Size: {:#x}",
                self.rec_begin as usize, self.rec_size);
        println!("Memory Begin: {:#x} -- Size: {:#x}",
                self.mem_begin as usize, self.tot_page * 4096);
        println!("------------Allocator Info Done------------");

    }
}
