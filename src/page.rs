use core::ptr;
use core::mem;
use core::array;

use crate::zone;

use crate::error::{KError, KErrorType};
use crate::new_kerror;

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

enum PageFlags{
    PF_FREE = (1 << 0),
    PF_TAKEN = (1 << 1)
}

struct PageMark{
    flags: PageFlags
}

struct PageRec{
    begin: *const u8,
    pg_off: usize,
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
    map_begin: *mut PageMark,
    map_size: usize,
    mem_begin: *const u8,
    mem_end: *const u8,
    rec_begin: *mut PageRec,
    rec_size: usize,
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

        //Pretty wild, but lets keep this since this is a **NAIVE** allocator
        if zone_size < 3 * PAGE_SIZE { 
            return Err(new_kerror!(KErrorType::ENOMEM));
        }

        let pmark_sz = mem::size_of::<PageMark>();
        let prec_sz = mem::size_of::<PageRec>();

        self.zone_begin = zone_start;
        self.zone_end = zone_end;

        self.mem_end = aligl_4k!(zone_end) as *const u8;
        self.map_begin = aligh_4k!(zone_start) as *mut PageMark;

        self.tot_page = unsafe{self.mem_end.offset_from(self.map_begin as *mut u8) as usize / PAGE_SIZE};
        self.map_size = aligh_4k!(self.tot_page * pmark_sz);
        self.rec_begin = unsafe{self.map_begin.add(self.map_size) as *mut PageRec};
        self.rec_size = aligh_4k!(self.tot_page * prec_sz);
        self.mem_begin = unsafe{self.rec_begin.add(self.rec_size) as *const u8};
        self.tot_page = unsafe{self.mem_end.offset_from(self.mem_begin) as usize / PAGE_SIZE};


        let map_elecnt = self.map_size / pmark_sz;

        for i in 0..map_elecnt{
            unsafe{
                self.map_begin.add(i).write(
                    PageMark{
                        flags: PageFlags::PF_FREE
                    }
                )
            }
        }

        let rec_elecnt =self.rec_size / prec_sz;

        for i in 0..rec_elecnt{
            unsafe{
                self.rec_begin.add(i).write(
                    PageRec{
                        begin: 0 as *const u8,
                        pg_off: 0,
                        len: 0,
                        inuse: false
                    }
                )
            }
        }

        self.print_info();

        Ok(())
    }

    fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError> {
        println!("Start allocate {} page(s)", pg_cnt);
        let mut alloc_addr;
        for i in 0..self.tot_page{
            match self.map_first_fit_avail(i, pg_cnt) {
                Ok(res) => {
                    if res == true {
                        self.map_mark_taken(i, pg_cnt);
                        alloc_addr = self.rec_add(i, pg_cnt)?;
                        return Ok(alloc_addr as *mut u8);
                    }else{
                        continue;
                    }
                },
                Err(reason) => {
                    return Err(reason);
                }
            }
        }

        Err(new_kerror!(KErrorType::ENOMEM))
        

    }

    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError> {
        println!("Start reclaiming...");
        let mut rec_arr = unsafe{core::slice::from_raw_parts_mut(self.rec_begin, self.rec_size)};

        let mut free_begin_pgnum: usize;
        let mut free_pgnum: usize;

        (free_begin_pgnum, free_pgnum) = self.rec_delete(addr)?;

        self.map_mark_free(free_begin_pgnum, free_pgnum);

        Ok(())

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

    fn map_first_fit_avail(&self, map_off: usize, thres_pg: usize) -> Result<bool, KError>{
        if map_off + thres_pg > self.tot_page {
            return Err(new_kerror!(KErrorType::ENOMEM));
        }
        let map_arr = unsafe{core::slice::from_raw_parts(self.map_begin.add(map_off), self.tot_page)};

        for (reg_cnt, reg) in map_arr.iter().enumerate() {
            if let PageFlags::PF_TAKEN = reg.flags{
                return Ok(false);
            }
        }

        Ok(true)

    }

    fn map_mark_taken(&mut self, map_off: usize, page_cnt: usize){
        let mut mark_begin = self.map_begin as *mut PageMark;
        mark_begin = mark_begin.wrapping_offset(map_off as isize);
        for i in 0..page_cnt{
            unsafe{
                mark_begin.add(i).write(
                    PageMark{
                        flags: PageFlags::PF_TAKEN
                    }
                )
            }
        }
    }

    fn map_mark_free(&mut self, map_off: usize, page_cnt: usize){
        let mut mark_begin = self.map_begin as *mut PageMark;
        mark_begin = mark_begin.wrapping_offset(map_off as isize);
        for i in 0..page_cnt{
            unsafe{
                mark_begin.add(i).write(
                    PageMark{
                        flags: PageFlags::PF_FREE
                    }
                )
            }
        }
    }

    fn rec_add(&mut self, map_off: usize, page_cnt: usize) -> Result<*const u8, KError>{
        let mut rec_arr;
        unsafe{
            rec_arr = core::slice::from_raw_parts_mut(self.rec_begin, self.rec_size);
        }

        for (rec_cnt, rec) in rec_arr.iter_mut().enumerate() {
            let addr = self.mem_begin.wrapping_byte_offset((map_off * PAGE_SIZE) as isize);
            if rec.inuse == false{
                *rec = PageRec{
                    begin: addr,
                    pg_off: map_off,
                    len: page_cnt,
                    inuse: true
                };
                return Ok(addr);
            }
        }

        Err(new_kerror!(KErrorType::ENOMEM))
    }

    fn rec_delete(&mut self, begin_addr: *const u8) -> Result<(usize, usize), KError>{
        let mut free_begin_pgnum: usize = 0;
        let mut free_pgnum: usize = 0;
        let mut found: bool = false;
        let rec_arr = unsafe{core::slice::from_raw_parts_mut(self.rec_begin, self.tot_page)};
        for (rec_cnt, rec) in rec_arr.iter_mut().enumerate() {
            if rec.begin == begin_addr{
                free_begin_pgnum = rec.pg_off;
                free_pgnum = rec.len;
                rec.inuse = false;

                found = true;
                break;
            }
        }

        if found == true{
            return Ok((free_begin_pgnum, free_pgnum));
        }else{
            return Err(new_kerror!(KErrorType::EFAULT));
        }
    }

}
