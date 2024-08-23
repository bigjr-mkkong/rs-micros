use crate::zone;
use core::ptr;
use core::mem;
use core::array;

pub const PAGE_SIZE:usize = 4096;

macro_rules! aligh_up_PGSIZE{
    ($n:expr) => {
        ($n + 4096 - 1) & !(4096 - 1)
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


#[derive(Clone, Copy)]
pub struct byte_allocator{
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

impl Default for byte_allocator{
    fn default() -> Self{
        byte_allocator{
            tot_page: 0,
            zone_begin: ptr::null_mut(),
            zone_end: ptr::null_mut(),
            map_begin: ptr::null_mut(),
            map_size: 0,
            mem_begin: ptr::null(),
            mem_end: ptr::null(),
            rec_begin: ptr::null_mut(),
            rec_size: 0
        }
    }
}

impl zone::page_allocator for byte_allocator{
    fn allocator_init(&mut self, zone_start: *mut u8, zone_end: *mut u8, zone_size: usize) {
        println!("Allocator initializing...");
        self.tot_page = zone_size / PAGE_SIZE;
        self.zone_begin = zone_start;
        self.zone_end = zone_end;
        self.mem_end = zone_end as *const u8;
        self.map_begin = zone_start;
        self.map_size = aligh_up_PGSIZE!(self.tot_page * mem::size_of::<PageMark>());
        unsafe{self.rec_begin = self.map_begin.add(self.map_size)};
        self.rec_size = aligh_up_PGSIZE!(self.tot_page * mem::size_of::<PageRec>());
        unsafe{self.mem_begin = self.rec_begin.add(self.rec_size) as *const u8};
        unsafe{self.tot_page = self.mem_end.offset_from(self.mem_begin) as usize / PAGE_SIZE};

    }

    fn alloc_pages(&mut self, pg_cnt: usize) -> Option<*mut u8> {
        println!("Start allocate {} page(s)", pg_cnt);
        None
    }

    fn free_pages(&mut self, addr: *mut u8) {
        println!("Start reclaiming...");
    }
}

