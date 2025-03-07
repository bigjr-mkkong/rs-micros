use core::array;
use core::mem;
use core::ptr;

use crate::kmem::{get_kheap_start, get_kheap_pgcnt, set_kheap_start};
use crate::cust_hmalloc;
use crate::zone;
use crate::zone::page_allocator;
use crate::{M_UART, S_UART};
use crate::alloc::collections::BTreeMap;

use crate::error::{KError, KErrorType};
use crate::new_kerror;

pub const PAGE_SIZE: usize = 4096;

#[macro_export]
macro_rules! aligh_4k {
    ($n:expr) => {
        ($n as usize + 4096 - 1) & !(4096 - 1)
    };
}

#[macro_export]
macro_rules! aligl_4k {
    ($n:expr) => {
        ($n as usize) & !(4096 - 1)
    };
}

#[macro_export]
macro_rules! addr2pfn {
    ($addr:expr) => {
        ($addr / PAGE_SIZE)
    };
}

/*
 * pgalloc_flags, pgalloc_mark, and pgalloc_rec keeps all *allocator specific info* which help
 * allocator to track the allocation
 *
 * PageFlags and Page Rec control the *page specific info* which help memory system to figure out
 * page state for each page(locate by pfn)
 *
 * They are independent from each other
 */
enum pgalloc_flags {
    PF_FREE = (1 << 0),
    PF_TAKEN = (1 << 1),
}

struct pgalloc_mark {
    flags: pgalloc_flags,
}

struct pgalloc_rec {
    begin: *const u8,
    pg_off: usize,
    len: usize,
    inuse: bool,
}

#[derive(Clone, Copy)]
pub enum PageFlags {
    DIRTY,
    LOCKED,
    DEFAULT,
}

#[derive(Clone, Copy)]
pub struct PageRec {
    pfn: usize,
    refcnt: usize,
    flag: PageFlags,
}
/*
 * naive_allocator needs at least 4 Pages memory to works
 */
pub struct naive_allocator {
    tot_page: usize,
    zone_begin: usize,
    zone_end: usize,
    map_begin: usize,
    map_size: usize,
    mem_begin: usize,
    mem_end: usize,
    rec_begin: usize,
    rec_size: usize,
    pagetree: Option<BTreeMap<usize, PageRec>>
}

impl page_allocator for naive_allocator {
    fn allocator_init(
        &mut self,
        zone_start: usize,
        zone_end: usize,
        zone_size: usize,
    ) -> Result<(usize, usize), KError> {
        //Pretty wild, but lets keep this since this is a **NAIVE** allocator
        Mprintln!("naive_Allocator Initializing");
        if zone_size < 3 * PAGE_SIZE {
            return Err(new_kerror!(KErrorType::ENOMEM));
        }

        let pmark_sz = mem::size_of::<pgalloc_mark>();
        let prec_sz = mem::size_of::<pgalloc_rec>();

        self.zone_begin = zone_start;
        self.zone_end = zone_end;

        self.mem_end = aligl_4k!(zone_end);
        self.map_begin = aligh_4k!(zone_start);

        self.tot_page = (self.mem_end - self.map_begin) / PAGE_SIZE;
        self.map_size = aligh_4k!(self.tot_page * pmark_sz);
        self.rec_begin = self.map_begin + self.map_size;

        self.rec_size = aligh_4k!(self.tot_page * prec_sz);
        self.mem_begin = self.rec_begin + self.rec_size;
        self.tot_page = (self.mem_end - self.mem_begin) / PAGE_SIZE;

        let map_elecnt = self.map_size / pmark_sz;
        let rawpt_mapbegin = self.map_begin as *mut pgalloc_mark;
        let rawpt_recbegin = self.rec_begin as *mut pgalloc_rec;
        let rawpt_membegin = self.mem_begin as *const u8;

        for i in 0..map_elecnt {
            unsafe {
                rawpt_mapbegin.add(i).write(pgalloc_mark {
                    flags: pgalloc_flags::PF_FREE,
                })
            }
        }

        let rec_elecnt = self.rec_size / prec_sz;

        for i in 0..rec_elecnt {
            unsafe {
                rawpt_recbegin.add(i).write(pgalloc_rec {
                    begin: 0 as *const u8,
                    pg_off: 0,
                    len: 0,
                    inuse: false,
                })
            }
        }

        self.print_info();

        Ok((self.map_begin, self.mem_begin))
    }

    fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError> {
        // Mprintln!("Start allocate {} page(s)", pg_cnt);
        let mut alloc_addr;
        for i in 0..self.tot_page {
            match self.map_first_fit_avail(i, pg_cnt) {
                Ok(res) => {
                    if res == true {
                        self.map_mark_taken(i, pg_cnt);
                        alloc_addr = self.rec_add(i, pg_cnt)?;

                        if let Some(_) = self.pagetree {
                            self.pagetree_update(&PageRec{
                                pfn: addr2pfn!(alloc_addr as usize),
                                refcnt: 1,
                                flag: PageFlags::DEFAULT,
                            });
                        } else {
                            let kheap_pgcnt = get_kheap_pgcnt();
                            set_kheap_start(alloc_addr as *mut u8);
                            let kheap_begin = get_kheap_start();
                            unsafe {
                                cust_hmalloc
                                    .lock()
                                    .init(kheap_begin as usize, kheap_pgcnt * PAGE_SIZE);
                            }
                            self.pagetree_init();

                            for pg_idx in 0..kheap_pgcnt{
                                unsafe{
                                    self.pagetree_update(&PageRec{
                                        pfn: addr2pfn!(alloc_addr.add(PAGE_SIZE * pg_idx) as usize),
                                        refcnt: 1,
                                        flag: PageFlags::DEFAULT,
                                    });
                                }
                            }
                        }

                        return Ok(alloc_addr as *mut u8);
                    } else {
                        continue;
                    }
                }
                Err(reason) => {
                    return Err(reason);
                }
            }
        }

        Err(new_kerror!(KErrorType::ENOMEM))
    }

    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError> {
        // Mprintln!("Start reclaiming...");
        let mut rec_arr = unsafe {
            core::slice::from_raw_parts_mut(self.rec_begin as *mut pgalloc_rec, self.rec_size)
        };

        let mut free_begin_pgnum: usize;
        let mut free_pgnum: usize;

        (free_begin_pgnum, free_pgnum) = self.rec_delete(addr)?;

        self.map_mark_free(free_begin_pgnum, free_pgnum);

        if let Some(_) = self.pagetree {
            self.pagetree_remove(addr2pfn!(addr as usize));
        }

        Ok(())
    }

}

impl naive_allocator {
    pub const fn new() -> Self {
        naive_allocator {
            tot_page: 0,
            zone_begin: 0,
            zone_end: 0,
            map_begin: 0,
            map_size: 0,
            rec_begin: 0,
            rec_size: 0,
            mem_begin: 0,
            mem_end: 0,
            pagetree: None
        }
    }
    fn print_info(&self) {
        Mprintln!("------------Allocator Info------------");
        Mprintln!("Total Pages: {}", self.tot_page);
        Mprintln!(
            "Mapping Begin: {:#x} -- Size: {:#x}",
            self.map_begin as usize,
            self.map_size
        );
        Mprintln!(
            "Record Begin: {:#x} -- Size: {:#x}",
            self.rec_begin as usize,
            self.rec_size
        );
        Mprintln!(
            "Memory Begin: {:#x} -- Size: {:#x}",
            self.mem_begin as usize,
            self.tot_page * 4096
        );
        Mprintln!("------------Allocator Info End------------");
    }

    fn map_first_fit_avail(&self, map_off: usize, thres_pg: usize) -> Result<bool, KError> {
        if map_off + thres_pg > self.tot_page {
            return Err(new_kerror!(KErrorType::ENOMEM));
        }
        let rawpt_mapbegin = self.map_begin as *mut pgalloc_mark;

        let map_arr =
            unsafe { core::slice::from_raw_parts(rawpt_mapbegin.add(map_off), self.tot_page) };

        for (reg_cnt, reg) in map_arr.iter().enumerate() {
            if let pgalloc_flags::PF_TAKEN = reg.flags {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn map_mark_taken(&mut self, map_off: usize, page_cnt: usize) {
        let mut mark_begin = self.map_begin as *mut pgalloc_mark;
        mark_begin = mark_begin.wrapping_offset(map_off as isize);
        for i in 0..page_cnt {
            unsafe {
                mark_begin.add(i).write(pgalloc_mark {
                    flags: pgalloc_flags::PF_TAKEN,
                })
            }
        }
    }

    fn map_mark_free(&mut self, map_off: usize, page_cnt: usize) {
        let mut mark_begin = self.map_begin as *mut pgalloc_mark;
        mark_begin = mark_begin.wrapping_offset(map_off as isize);
        for i in 0..page_cnt {
            unsafe {
                mark_begin.add(i).write(pgalloc_mark {
                    flags: pgalloc_flags::PF_FREE,
                })
            }
        }
    }

    fn rec_add(&mut self, map_off: usize, page_cnt: usize) -> Result<*const u8, KError> {
        let mut rec_arr;
        unsafe {
            rec_arr =
                core::slice::from_raw_parts_mut(self.rec_begin as *mut pgalloc_rec, self.rec_size);
        }

        let rawpt_membegin = self.mem_begin as *mut u8;
        for (rec_cnt, rec) in rec_arr.iter_mut().enumerate() {
            let addr = rawpt_membegin.wrapping_byte_offset((map_off * PAGE_SIZE) as isize);
            if rec.inuse == false {
                *rec = pgalloc_rec {
                    begin: addr,
                    pg_off: map_off,
                    len: page_cnt,
                    inuse: true,
                };
                return Ok(addr);
            }
        }

        Err(new_kerror!(KErrorType::ENOMEM))
    }

    fn rec_delete(&mut self, begin_addr: *const u8) -> Result<(usize, usize), KError> {
        let mut free_begin_pgnum: usize = 0;
        let mut free_pgnum: usize = 0;
        let mut found: bool = false;
        let rawpt_recbegin = self.rec_begin as *mut pgalloc_rec;

        let rec_arr = unsafe { core::slice::from_raw_parts_mut(rawpt_recbegin, self.tot_page) };
        for (rec_cnt, rec) in rec_arr.iter_mut().enumerate() {
            if rec.begin == begin_addr {
                free_begin_pgnum = rec.pg_off;
                free_pgnum = rec.len;
                rec.inuse = false;

                found = true;
                break;
            }
        }

        if found == true {
            return Ok((free_begin_pgnum, free_pgnum));
        } else {
            return Err(new_kerror!(KErrorType::EFAULT));
        }
    }

    fn pagetree_init(&mut self) {
        self.pagetree = Some(BTreeMap::<usize, PageRec>::new());
    }

    fn pagetree_update(&mut self, newpg: &PageRec) -> Result<(), KError>{
        match self.pagetree {
            Some(ref mut pgtree) => {
                pgtree.insert(newpg.pfn, *newpg);
                Ok(())
            },
            None => {
                panic!();
                Err(new_kerror!(KErrorType::EFAULT))
            }
        }
    }

    fn pagetree_remove(&mut self, pfn: usize) -> Result<(usize, PageRec), KError>{
        match self.pagetree {
            Some(ref mut pgtree) => {
                Ok(pgtree.remove_entry(&pfn).unwrap())
            },
            None => {
                Err(new_kerror!(KErrorType::EFAULT))
            }
        }
    }

    fn pagetree_get(&self, pfn: usize) -> Option<PageRec> {
        match self.pagetree {
            Some(ref pgtree) => {
                let ret = pgtree.get(&pfn);
                if let Some(pgrec_ref) = ret {
                    Some(*pgrec_ref)
                } else{
                    None
                }
            },
            None => {
                None
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct empty_allocator {
    place_holder: usize,
}

impl empty_allocator {
    pub const fn new() -> Self {
        empty_allocator {
            place_holder: 0xdeadbeef,
        }
    }
}

impl page_allocator for empty_allocator {
    fn allocator_init(
        &mut self,
        zone_start: usize,
        zone_end: usize,
        zone_size: usize,
    ) -> Result<(usize, usize), KError> {
        Mprintln!("Placeholder Allocator Initializing");
        Ok((0 as usize, 0 as usize))
    }
    fn alloc_pages(&mut self, pg_cnt: usize) -> Result<*mut u8, KError> {
        Err(new_kerror!(KErrorType::ENOSYS))
    }
    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError> {
        Err(new_kerror!(KErrorType::ENOSYS))
    }
}

