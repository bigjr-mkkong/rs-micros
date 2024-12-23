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

    unsafe fn realloc(
    &self,
    ptr: *mut u8,
    layout: Layout,
    new_size: usize) -> *mut u8 {

    if new_size == 0 {
        self.dealloc(ptr, layout);
        return core::ptr::null_mut();
    }

    if ptr.is_null() {
        return self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());
    }

    let new_layout = Layout::from_size_align(new_size, layout.align());
    let new_ptr = match new_layout {
        Ok(layout) => self.alloc(layout),
        Err(_) => return core::ptr::null_mut(),
    };

    if new_ptr.is_null() {
        return core::ptr::null_mut();
    }

    let copy_size = core::cmp::min(layout.size(), new_size);
    core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);

    self.dealloc(ptr, layout);

    new_ptr
}

unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    // Allocate memory
    let ptr = self.alloc(layout);

    // If allocation failed, return null
    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    // Zero the memory
    core::ptr::write_bytes(ptr, 0, layout.size());

    ptr
}

}
