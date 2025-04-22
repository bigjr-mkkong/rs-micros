use crate::{Mprintln, Sprintln};
use core::array;
use core::mem;
use core::ptr;

use crate::alloc::collections::BTreeMap;
use crate::cust_hmalloc;
use crate::kmem::{get_kheap_pgcnt, get_kheap_start, set_kheap_start};
use crate::zone;
use crate::zone::page_allocator;
use crate::{M_UART, S_UART};

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
//TODO remove "PF"
enum pgalloc_flags {
    PF_FREE = (1 << 0),
    PF_TAKEN = (1 << 1),
}

struct pgalloc_mark {
    flags: pgalloc_flags,
}

// struct pgalloc_rec {
//     begin: *const u8,
//     pg_off: usize,
//     len: usize,
//     inuse: bool,
// }

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
    pagetree: Option<BTreeMap<usize, PageRec>>,
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

        self.zone_begin = zone_start;
        self.zone_end = zone_end;

        self.mem_end = aligl_4k!(zone_end);
        self.map_begin = aligh_4k!(zone_start);

        self.tot_page = (self.mem_end - self.map_begin) / PAGE_SIZE;
        self.map_size = aligh_4k!(self.tot_page * pmark_sz);
        // self.rec_begin = self.map_begin + self.map_size;

        // self.rec_size = aligh_4k!(self.tot_page * prec_sz);
        self.mem_begin = self.map_begin + self.map_size;
        self.tot_page = (self.mem_end - self.mem_begin) / PAGE_SIZE;

        let map_elecnt = self.map_size / pmark_sz;
        let rawpt_mapbegin = self.map_begin as *mut pgalloc_mark;
        let rawpt_membegin = self.mem_begin as *const u8;

        for i in 0..map_elecnt {
            unsafe {
                rawpt_mapbegin.add(i).write(pgalloc_mark {
                    flags: pgalloc_flags::PF_FREE,
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

                        alloc_addr = (self.mem_begin + (i * PAGE_SIZE)) as *const u8;

                        if let Some(_) = self.pagetree {
                            for pg_idx in 0..pg_cnt {
                                unsafe {
                                    self.pagetree_update(&PageRec {
                                        pfn: addr2pfn!(alloc_addr.add(PAGE_SIZE * pg_idx) as usize),
                                        refcnt: 1,
                                        flag: PageFlags::DEFAULT,
                                    });
                                }
                            }
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

                            for pg_idx in 0..kheap_pgcnt {
                                unsafe {
                                    self.pagetree_update(&PageRec {
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

    /*
     * NOTE
     * alloc_pages() can allocate multiple continuous paddr pages, but free_pages can only free one
     * page at a time.
     */
    fn free_pages(&mut self, addr: *mut u8) -> Result<(), KError> {
        // Mprintln!("Start reclaiming...");
        let pfn = addr2pfn!(addr as usize);
        let refcnt = self
            .pagetree_getrefcnt(pfn)
            .ok_or(new_kerror!(KErrorType::EFAULT))?;

        if refcnt > 1 {
            self.pagetree_setrefcnt(pfn, refcnt - 1);
            Ok(())
        } else {
            let mut free_begin_pgnum: usize;
            let mut free_pgnum: usize;

            free_begin_pgnum = (addr as usize - self.mem_begin) / PAGE_SIZE;

            self.map_mark_free(free_begin_pgnum, 1);

            self.pagetree_remove(pfn);

            Ok(())
        }
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
            mem_begin: 0,
            mem_end: 0,
            pagetree: None,
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

    fn pagetree_init(&mut self) {
        self.pagetree = Some(BTreeMap::<usize, PageRec>::new());
    }

    fn pagetree_update(&mut self, newpg: &PageRec) -> Result<(), KError> {
        match self.pagetree {
            Some(ref mut pgtree) => {
                pgtree.insert(newpg.pfn, *newpg);
                Ok(())
            }
            None => {
                panic!();
                Err(new_kerror!(KErrorType::EFAULT))
            }
        }
    }

    fn pagetree_remove(&mut self, pfn: usize) -> Result<(usize, PageRec), KError> {
        match self.pagetree {
            Some(ref mut pgtree) => Ok(pgtree.remove_entry(&pfn).unwrap()),
            None => Err(new_kerror!(KErrorType::EFAULT)),
        }
    }

    fn pagetree_get(&self, pfn: usize) -> Option<PageRec> {
        self.pagetree.as_ref()?.get(&pfn).copied()
    }

    fn pagetree_getrefcnt(&self, pfn: usize) -> Option<usize> {
        self.pagetree.as_ref()?.get(&pfn).map(|pgrec| pgrec.refcnt)
    }

    fn pagetree_setrefcnt(&mut self, pfn: usize, newrefcnt: usize) -> Option<()> {
        self.pagetree
            .as_mut()?
            .get_mut(&pfn)
            .map(|pgrec| pgrec.refcnt = newrefcnt)
    }

    fn pagetree_getflag(&self, pfn: usize) -> Option<PageFlags> {
        self.pagetree.as_ref()?.get(&pfn).map(|pgrec| pgrec.flag)
    }

    fn pagetree_setflag(&mut self, pfn: usize, newflag: PageFlags) -> Option<()> {
        self.pagetree
            .as_mut()?
            .get_mut(&pfn)
            .map(|pgrec| pgrec.flag = newflag)
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
