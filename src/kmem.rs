use crate::error::{KError, KErrorType};
use crate::zone::{kmalloc_page, kfree_page, zone_type};
use crate::vm::PageTable;

static mut KHEAP_START: *mut u8 = 0 as *mut u8;
static mut KHEAP_PGCNT: usize = 64;
static mut KMEM_PAGE_TABLE: *mut PageTable = 0 as *mut PageTable;

pub fn init() -> Result<(), KError>{
    unsafe{
        KHEAP_START = kmalloc_page(zone_type::ZONE_NORMAL, KHEAP_PGCNT)?;
        KMEM_PAGE_TABLE = kmalloc_page(zone_type::ZONE_NORMAL, 1)? as *mut PageTable;
    }

    Ok(())
}

pub fn get_kheap_start() -> *mut u8{
    unsafe{KHEAP_START}
}

pub fn get_page_table() -> *mut PageTable{
    unsafe{KMEM_PAGE_TABLE}
}

pub fn get_kheap_pgcnt() -> usize{
    unsafe{KHEAP_PGCNT}
}
