use core::alloc::{GlobalAlloc, Layout};
/*
 * We are not going to make a fancy lock-free, high-throughput allocator
 * Just a simple one would be enough
 */

extern "C" {
    fn AllocatorInit(begin: usize, len: usize) -> usize;
    fn malloc(sz: usize, align: usize) -> *mut u8;
    fn free(pt: *mut u8, sz: usize, align: usize);
}

pub struct custom_alloc;

impl custom_alloc {
    pub fn new(begin_addr: usize, length: usize) -> usize {
        unsafe { AllocatorInit(begin_addr, length) }
    }
}

unsafe impl GlobalAlloc for custom_alloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        free(ptr, layout.size(), layout.align());
    }
}
