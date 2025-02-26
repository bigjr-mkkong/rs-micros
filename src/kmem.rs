use crate::error::{KError, KErrorType};
use crate::vm::PageTable;
use crate::zone::{kfree_page, kmalloc_page, zone_type};

static mut KHEAP_START: *mut u8 = 0 as *mut u8;
static mut KHEAP_PGCNT: usize = 256;
static mut KMEM_PAGE_TABLE: *mut PageTable = 0 as *mut PageTable;
static mut KERN_SATP: u64 = 0;

pub fn init() -> Result<(), KError> {
    unsafe {
        /*
         * KHEAP_START are been allocated before settle down of kheap
         * which means there's no way to insert pageinfo into pagetree
         * I may need to insert it after, which means i need to pass
         * out the page range from this kmalloc_page. A good solution
         * may be kmalloc_page check if pagetree is None. It doesn't
         * mess with it if it is None, otherwise it insert information
         */
        KHEAP_START = kmalloc_page(zone_type::ZONE_NORMAL, KHEAP_PGCNT)?;
        KMEM_PAGE_TABLE = kmalloc_page(zone_type::ZONE_NORMAL, 1)? as *mut PageTable;
    }

    Ok(())
}

pub fn get_kheap_start() -> *mut u8 {
    unsafe { KHEAP_START }
}

pub fn get_page_table() -> *mut PageTable {
    unsafe { KMEM_PAGE_TABLE }
}

pub fn get_kheap_pgcnt() -> usize {
    unsafe { KHEAP_PGCNT }
}

pub fn get_ksatp() -> u64 {
    unsafe { KERN_SATP }
}

pub fn set_ksatp(new_satp: u64) {
    unsafe { KERN_SATP = new_satp }
}
