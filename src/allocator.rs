use core::alloc::{GlobalAlloc, Layout};
use crate::cust_hmalloc;
/*
 * We are not going to make a fancy lock-free, high-throughput allocator
 * Just a simple one would be enough
 */

extern "C" {
    fn kheap_AllocatorInit(begin: usize, len: usize) -> usize;
    fn kheap_malloc(sz: usize, align: usize) -> *mut u8;
    fn kheap_free(pt: *mut u8, sz: usize, align: usize);
}

pub struct kheap_alloc;

impl kheap_alloc{
    pub const fn new() -> Self{
        Self{}
    }
}


pub struct custom_kheap_malloc;

impl custom_kheap_malloc {
    pub const fn new() -> Self{
        Self{}
    }

    pub fn init(&self, begin_addr: usize, length: usize) -> usize {
        unsafe { kheap_AllocatorInit(begin_addr, length) }
    }

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        kheap_malloc(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        kheap_free(ptr, layout.size(), layout.align());
    }
}

/*
 * In order to fullfill multicore safety, kheap_alloc needs to call custom_kheap_malloc
 */
unsafe impl GlobalAlloc for kheap_alloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        cust_hmalloc.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        cust_hmalloc.lock().dealloc(ptr, layout)
    }
}
