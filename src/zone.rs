use core::mem;
use core::array;

use crate::SYS_UART;
use crate::SYS_ZONES;
use crate::error::{KError, KErrorType};
use crate::new_kerror;

use crate::page::{naive_allocator, empty_allocator};

#[derive(Clone, Copy)]
pub enum AllocatorSelector{
    EmptyAllocator,
    NaiveAllocator
}

#[derive(Clone, Copy)]
pub enum Allocators{
    EmptyAllocator(empty_allocator),
    NaiveAllocator(naive_allocator)
}

impl page_allocator for Allocators{
    fn allocator_init(&mut self, zone_start: usize, zone_end: usize, zone_size: usize) -> Result<(), KError>{
        match self{
            Allocators::EmptyAllocator(alloc) => alloc.allocator_init(zone_start, zone_end, zone_size),
            Allocators::NaiveAllocator(alloc) => alloc.allocator_init(zone_start, zone_end, zone_size),
        }
    }

    fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError>{
        match self{
            Allocators::EmptyAllocator(alloc) => alloc.alloc_pages(pg_cnt),
            Allocators::NaiveAllocator(alloc) => alloc.alloc_pages(pg_cnt),
        }
    }
    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError>{
        match self{
            Allocators::EmptyAllocator(alloc) => alloc.free_pages(addr),
            Allocators::NaiveAllocator(alloc) => alloc.free_pages(addr),
        }
    }
}

#[derive(Clone, Copy)]
pub enum zone_type{
    ZONE_UNDEF = (0),
    ZONE_NORMAL = (1),
    ZONE_VIRTIO = (2)
}

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

    pub fn val(&self) -> usize{
        *self as usize
    }

    pub const fn type_cnt() -> usize{
        3
    }

}

pub trait page_allocator{
    fn allocator_init(&mut self, zone_start: usize, zone_end: usize, zone_size: usize) -> Result<(), KError>;
    fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError>;
    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError>;
}

#[derive(Clone, Copy)]
pub struct mem_zone{
    begin_addr: usize,
    end_addr: usize,
    zone_size: usize,
    types: zone_type,
    // pg_allocator: Option<A>
    pg_allocator: Option<Allocators>
}

impl mem_zone{
    pub const fn new() -> Self{
        mem_zone{
            begin_addr: 0,
            end_addr: 0,
            zone_size: 0,
            types: zone_type::ZONE_UNDEF,
            pg_allocator: None
        }
    }

    pub fn init(&mut self, _start: *const u8, _end: *const u8, _type: zone_type, allocator: AllocatorSelector) -> Result<(), KError>{

        self.begin_addr = _start as usize;
        self.end_addr = _end as usize;
        self.zone_size = (unsafe{_end.offset_from(_start)}) as usize;
        self.types = _type;
        let mut allocator = match allocator{
            AllocatorSelector::EmptyAllocator => Allocators::EmptyAllocator(empty_allocator::new()),
            AllocatorSelector::NaiveAllocator => Allocators::NaiveAllocator(naive_allocator::new())
        };
        allocator.allocator_init(_start as usize, _end as usize, self.zone_size)?;

        self.pg_allocator = Some(allocator);
        Ok(())
    }

    pub fn get_size(&self) -> Result<usize, KError>{
        if self.begin_addr > self.end_addr{
            return Err(new_kerror!(KErrorType::ENOMEM));
        } else{
            return Ok(self.end_addr - self.begin_addr);
        }
    }

    pub fn print_all(&self) {
        println!("[ZONE INFO] Begin: {:#x} -> End: {:#x}  Size: {:#x}  Type: {:#?}",
            self.begin_addr as usize,
            self.end_addr as usize,
            self.zone_size,
            self.types.as_str());
    }

    pub fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError> {
        if let Some(mut alloc) = self.pg_allocator{
            alloc.alloc_pages(pg_cnt)
        }else{
            Err(new_kerror!(KErrorType::ENOSYS))
        }
    }

    pub fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError> {
        if let Some(mut alloc) = self.pg_allocator{
            alloc.free_pages(addr)
        }else{
            Err(new_kerror!(KErrorType::ENOSYS))
        }
    }
}

pub fn kmalloc_page(ztype: zone_type, pg_cnt: usize) -> Result<*mut u8, KError>{
    SYS_ZONES[ztype.val()].lock().alloc_pages(pg_cnt)
}

pub fn kfree_page(ztype: zone_type, addr: *mut u8) -> Result<(), KError> {
    SYS_ZONES[ztype.val()].lock().free_pages(addr)
}
