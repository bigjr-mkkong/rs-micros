use crate::zone;

pub const PAGE_SIZE:usize = 4096;

enum page_flag{
    PF_FREE = (1 << 0),
    PF_TAKEN = (1 << 1)
}

struct Page{
    flags: u8
}

#[derive(Clone, Copy, Default)]
pub struct byte_allocator{
    tot_page: usize
}

impl byte_allocator{
    pub fn new() -> Self{
        byte_allocator{
            tot_page: 0
        }
    }
}

impl zone::page_allocator for byte_allocator{
    fn allocator_init(&mut self, zone_start: *mut u8, zone_end: *mut u8, zone_size: usize) {
        println!("Allocator initializing...");
        self.tot_page = zone_size / PAGE_SIZE;
    }

    fn alloc_pages(&mut self, pg_cnt: usize) -> Option<*mut u8> {
        println!("Start allocate {} page(s)", pg_cnt);
        None
    }

    fn free_pages(&mut self, addr: *mut u8) {
        println!("Start reclaiming...");
    }
}

