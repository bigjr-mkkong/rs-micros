use crate::cpu::flush_tlb;
use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::page;
use crate::zone::{kfree_page, kmalloc_page, zone_type};
use crate::{aligh_4k, aligl_4k};
use crate::{M_UART, S_UART};

pub struct PageTable {
    pub entries: [PageEntry; 512],
}

pub struct PageEntry {
    pub entry: i64,
}

#[repr(i64)]
#[derive(Copy, Clone)]
pub enum EntryBits {
    None = 0,
    Valid = 1 << 0,
    Read = 1 << 1,
    Write = 1 << 2,
    Execute = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Access = 1 << 6,
    Dirty = 1 << 7,

    // Convenience combinations
    ReadWrite = 1 << 1 | 1 << 2,
    ReadExecute = 1 << 1 | 1 << 3,
    ReadWriteExecute = 1 << 1 | 1 << 2 | 1 << 3,

    // User Convenience Combinations
    UserReadWrite = 1 << 1 | 1 << 2 | 1 << 4,
    UserReadExecute = 1 << 1 | 1 << 3 | 1 << 4,
    UserReadWriteExecute = 1 << 1 | 1 << 2 | 1 << 3 | 1 << 4,
}

impl EntryBits {
    pub fn val(self) -> i64 {
        self as i64
    }
}

impl PageTable {
    fn len() -> usize {
        512
    }
}

impl PageEntry {
    pub fn get_entry(&self) -> i64 {
        self.entry
    }

    pub fn set_entry(&mut self, new_ent: i64) {
        self.entry = new_ent;
    }

    pub fn is_valid(&self) -> bool {
        self.get_entry() & EntryBits::Valid.val() != 0
    }

    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }

    pub fn is_leaf(&self) -> bool {
        self.get_entry() & EntryBits::ReadWriteExecute.val() != 0
    }

    pub fn is_branch(&self) -> bool {
        !self.is_leaf()
    }
}

/*
 * VADDR format:
 * [x_xxxx_xxxx] [x_xxxx_xxxx] [x_xxxx_xxxx] [xxxx_xxxx_xxxx]
 *     VPN[2]       VPN[1]         VPN[0]         Offset
 */
pub fn mem_map(
    root: &mut PageTable,
    vaddr: usize,
    paddr: usize,
    bits: i64,
    level: usize,
) -> Result<(), KError> {
    if bits & EntryBits::ReadWriteExecute.val() == 0 {
        return Err(new_kerror!(KErrorType::EFAULT));
    }

    let vpn = [
        (vaddr >> 12) & 0x1ff,
        (vaddr >> 21) & 0x1ff,
        (vaddr >> 30) & 0x1ff,
    ];

    let ppn = [
        (paddr >> 12) & 0x1ff,
        (paddr >> 21) & 0x1ff,
        (paddr >> 30) & 0x3ff_ffff,
    ];

    let mut v = &mut root.entries[vpn[2]];

    for i in (level..2).rev() {
        if !v.is_valid() {
            let page = kmalloc_page(zone_type::ZONE_NORMAL, 1)?;

            v.set_entry(((page as i64) >> 2) | EntryBits::Valid.val());
        }

        let entry = ((v.get_entry() & !0x3ff) << 2) as *mut PageEntry;
        v = unsafe { entry.add(vpn[i]).as_mut().unwrap() };
    }

    let entry = (ppn[2] << 28) as i64
        | (ppn[1] << 19) as i64
        | (ppn[0] << 10) as i64
        | bits
        | EntryBits::Valid.val();

    v.set_entry(entry);
    Ok(())
}

pub fn mem_unmap(root: &mut PageTable, vaddr: usize, level: usize) -> Result<(), KError> {
    let vpn = [
        (vaddr >> 12) & 0x1ff,
        (vaddr >> 21) & 0x1ff,
        (vaddr >> 30) & 0x1ff,
    ];

    let mut v = &mut root.entries[vpn[2]];

    for i in (0..=2).rev() {
        if v.is_invalid() {
            break;
        } else if v.is_leaf() {
            let new_ent = v.get_entry();
            v.set_entry(new_ent & !EntryBits::Valid.val());
            flush_tlb();

            return Ok(());
        }

        let entry = ((v.get_entry() & !0x3ff) << 2) as *mut PageEntry;
        v = unsafe { entry.add(vpn[i - 1]).as_mut().unwrap() };
    }

    Ok(())
}

pub fn virt2phys(root: &PageTable, vaddr: usize) -> Result<Option<usize>, KError> {
    let vpn = [
        (vaddr >> 12) & 0x1ff,
        (vaddr >> 21) & 0x1ff,
        (vaddr >> 30) & 0x1ff,
    ];

    let mut v = &root.entries[vpn[2]];

    for i in (0..=2).rev() {
        if v.is_invalid() {
            break;
        } else if v.is_leaf() {
            let off_mask = (1 << (12 + i * 9)) - 1;
            let vaddr_pgoff = vaddr & off_mask;
            let addr = ((v.get_entry() << 2) as usize) & !off_mask;

            return Ok(Some(addr | vaddr_pgoff));
        }

        let entry = ((v.get_entry() & !0x3ff) << 2) as *const PageEntry;
        v = unsafe { entry.add(vpn[i - 1]).as_ref().unwrap() };
    }

    Ok(None)
}

pub fn ident_range_map(
    root: &mut PageTable,
    begin: usize,
    end: usize,
    bits: i64,
) -> Result<(), KError> {
    let mut addr_begin = aligl_4k!(begin);
    let mut addr_end = aligh_4k!(end);

    let range_pgcnt = (addr_end - addr_begin) / page::PAGE_SIZE;

    Mprintln!(
        "Identical Map PADDR range: {:#x} -> {:#x}",
        addr_begin,
        addr_end
    );

    for _ in 0..range_pgcnt {
        mem_map(root, addr_begin, addr_begin, bits, 0);
        addr_begin += page::PAGE_SIZE;
    }

    Ok(())
}

pub fn range_unmap(root: &mut PageTable, begin: usize, end: usize) -> Result<(), KError> {
    let mut addr_begin = aligl_4k!(begin);
    let mut addr_end = aligh_4k!(end);

    let range_pgcnt = (addr_end - addr_begin) / page::PAGE_SIZE;

    Mprintln!("Unmap addr range: {:#x} -> {:#x}", addr_begin, addr_end);

    for _ in 0..range_pgcnt {
        mem_unmap(root, addr_begin, 0);
        addr_begin += page::PAGE_SIZE;
    }

    Ok(())
}
