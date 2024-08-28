use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::sys_uart;

pub struct PageTable{
    pub entries: [PageEntry; 512]
}

pub struct PageEntry{
    pub entry: i64
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

impl EntryBits{
    fn val(self) -> i64{
        self as i64
    }
}


impl PageTable{
    fn len() -> usize{
        512
    }
}

impl PageEntry{
    pub fn get_entry(&self) -> i64{
        self.entry
    }

    pub fn set_entry(&mut self, new_ent: i64) {
        self.entry = new_ent;
    }

    pub fn is_valid(&self) -> bool{
        self.get_entry() & EntryBits::Valid.val() != 0
    }

    pub fn is_invalid(&self) -> bool{
        !self.is_valid()
    }

    pub fn is_leaf(&self) -> bool{
        self.get_entry() & EntryBits::ReadWriteExecute.val() != 0
    }

    pub fn is_branch(&self) ->bool{
        !self.is_leaf()
    }

}

/*
 * VADDR format:
 * [x_xxxx_xxxx] [x_xxxx_xxxx] [x_xxxx_xxxx] [xxxx_xxxx_xxxx]
 *     VPN[2]       VPN[1]         VPN[0]         Offset
 */
pub fn mmap(root: &mut PageTable, vaddr: usize, paddr: usize, bits: i64, level: usize) -> 
    Result<(), KError>{
        if bits & EntryBits::ReadWriteExecute.val() == 0{
            return Err(new_kerror!(KErrorType::EFAULT));
        }

        let vpn = [(vaddr >> 12) & 0x1ff,
                    (vaddr >> 21) & 0x1ff,
                    (vaddr >> 30) & 0x1ff];

        let ppn = [(paddr >> 12) & 0x1ff,
                    (paddr >> 21) & 0x1ff,
                    (paddr >> 30) & 0x3ff_ffff];

        let mut v = &mut root.entries[vpn[2]];

        Ok(())
}
