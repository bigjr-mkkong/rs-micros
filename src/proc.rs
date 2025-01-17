use crate::alloc::boxed::Box;
use crate::alloc::vec::Vec;
use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, mscratch_write, satp_write,
    sscratch_write, which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall;
use crate::ecall::S2Mop;
use crate::error::{KError, KErrorType};
use crate::kmem::{get_ksatp, get_page_table};
use crate::new_kerror;
use crate::page::PAGE_SIZE;
use crate::vm::{ident_range_map, range_unmap, mem_map, EntryBits, PageEntry, PageTable};
use crate::zone::{kfree_page, kmalloc_page, zone_type};
use crate::KERNEL_TRAP_FRAME;
use crate::{M_UART, S_UART};
use core::cell::UnsafeCell;
use riscv::register::{mstatus, sstatus};

#[derive(Clone, Copy)]
pub enum task_state {
    Ready,
    Running,
    Block,
    Zombie,
    Dead,
}

#[derive(Clone, Copy)]
pub enum task_typ {
    KERN_TASK,
    USER_TASK,
}

const KTASK_STACK_SZ: usize = 1 * PAGE_SIZE;
const KTASK_EXPSTACK_SZ: usize = 1 * PAGE_SIZE;
/*
 * cpu need to keep same as current hartid
 * We can have per-cpu schedule queue, and each task in a single queue need to have same cpu value
 */
pub struct task_struct {
    trap_frame: TrapFrame,
    state: task_state,
    pc: usize,
    stack_base: usize,
    exp_stack_base: usize,
    cpu: usize,
    pid: usize,
    typ: task_typ,
}

impl Drop for task_struct{
    fn drop(&mut self){
        let pageroot_ptr = get_page_table();
        let mut pageroot = unsafe { pageroot_ptr.as_mut().unwrap() };

        let kt_stack_begin: *mut u8 = (self.stack_base - KTASK_STACK_SZ) as *mut u8;
        kfree_page(zone_type::ZONE_NORMAL, kt_stack_begin);
        range_unmap(pageroot,
                    kt_stack_begin as usize,
                    self.stack_base);

        let exp_stack_begin: *mut u8 = (self.exp_stack_base - KTASK_EXPSTACK_SZ) as *mut u8;
        kfree_page(zone_type::ZONE_NORMAL, exp_stack_begin);
        range_unmap(pageroot,
                    exp_stack_begin as usize,
                    self.exp_stack_base);


    }
}

impl task_struct {
    pub const fn new() -> Self {
        Self {
            trap_frame: TrapFrame::new(),
            state: task_state::Ready,
            pc: 0 as usize,
            cpu: 0 as usize,
            stack_base: 0 as usize,
            exp_stack_base: 0 as usize,
            pid: 0 as usize,
            typ: task_typ::KERN_TASK,
        }
    }

    pub fn set_state(&mut self, new_state: task_state) {
        self.state = new_state;
    }

    pub fn init(&mut self, func: usize) -> Result<usize, KError> {
        self.cpu = which_cpu();
        self.trap_frame.cpuid = self.cpu;
        if let task_typ::KERN_TASK = self.typ {
            self.trap_frame.satp = get_ksatp() as usize;
            let pageroot_ptr = get_page_table();
            let mut pageroot = unsafe { pageroot_ptr.as_mut().unwrap() };
            //initialize kernel task
            unsafe {
                self.pc = func;
                self.pid = 0;

                let kt_stack = kmalloc_page(zone_type::ZONE_NORMAL, KTASK_STACK_SZ / PAGE_SIZE)?
                    .add(KTASK_STACK_SZ);
                ident_range_map(
                    pageroot,
                    kt_stack.sub(KTASK_STACK_SZ) as usize,
                    kt_stack as usize,
                    EntryBits::ReadWrite.val(),
                );

                let kt_expstack =
                    kmalloc_page(zone_type::ZONE_NORMAL, KTASK_EXPSTACK_SZ / PAGE_SIZE)?
                        .add(KTASK_EXPSTACK_SZ);
                ident_range_map(
                    pageroot,
                    kt_expstack.sub(KTASK_EXPSTACK_SZ) as usize,
                    kt_expstack as usize,
                    EntryBits::ReadWrite.val(),
                );

                self.stack_base = (kt_stack as usize);
                self.exp_stack_base = kt_expstack as usize;

                self.trap_frame.trap_stack = (self.exp_stack_base - 1) as *mut u8;
                self.trap_frame.regs[2] = self.stack_base - 1;
            }
        } else {
            //initialize user task
            let pg_root_ptr = kmalloc_page(zone_type::ZONE_NORMAL, 1)? as *mut PageTable;
            let pg_root = unsafe { pg_root_ptr.as_mut().unwrap() };

            let satp_root = pg_root_ptr as usize;
            self.trap_frame.satp = make_satp(SATP_mode::Sv39, 0, satp_root);

            unsafe {
                self.pc = func;

                // allocate trap stack
                let trap_stack = kmalloc_page(zone_type::ZONE_NORMAL, 2)?.add(PAGE_SIZE * 2);
                ident_range_map(
                    pg_root,
                    trap_stack.sub(2 * PAGE_SIZE) as usize,
                    trap_stack as usize,
                    EntryBits::ReadWrite.val(),
                );

                //allocate text segment
                let text_mem = kmalloc_page(zone_type::ZONE_NORMAL, 1)? as *mut usize;
                let prog_begin = self.pc as *mut usize;
                prog_begin.copy_to_nonoverlapping(text_mem, 3);
                mem_map(
                    pg_root,
                    self.pc,
                    text_mem as usize,
                    EntryBits::Execute.val(),
                    0,
                );

                //allocate execution stack
                let exe_stack = kmalloc_page(zone_type::ZONE_NORMAL, 1)?.add(PAGE_SIZE * 1);
                ident_range_map(
                    pg_root,
                    exe_stack.sub(1 * PAGE_SIZE) as usize,
                    exe_stack as usize,
                    EntryBits::ReadWrite.val(),
                );
                //setup SP
                self.trap_frame.regs[2] = exe_stack as usize;
            }
        }

        Ok(0)
    }
    /*
     * task.save will save records from KERNL_TRAP_FRAME to specific task's trapframe
     * infoe KERNEL_TRAP_FRAME.regs are guarantee to be the cpu state before trapping
     * so we can directly copy the contents on it
     *
     * Scheduler(Or at least, sched() function is going to run in M-mode)
     */
    pub fn save(&mut self) {
        let cur_cpu = self.cpu;
        unsafe {
            //switch to kernel trap frame
            // sscratch_write((&KERNEL_TRAP_FRAME[cur_cpu] as *const TrapFrame) as usize);
            self.trap_frame.refresh_from(&KERNEL_TRAP_FRAME[cur_cpu]);
        }
    }

    pub fn set_pc(&mut self, newpc: usize) {
        self.pc = newpc;
    }

    pub fn resume_from_M(&mut self) {
        let next_pc = self.pc;
        unsafe {
            match self.typ {
                task_typ::KERN_TASK => {
                    sstatus::set_spp(sstatus::SPP::Supervisor);
                }
                task_typ::USER_TASK => {
                    sstatus::set_spp(sstatus::SPP::User);
                }
            }
            let tasktrap_addr = &self.trap_frame as *const TrapFrame;
            let task_s1val = KERNEL_TRAP_FRAME[self.cpu].regs[8];

            sscratch_write(tasktrap_addr as usize);

            /*
             * resume_from_M only callable from M mode trap handler,
             * this line is responsible to restore the original mscratch value(KERNEL_TRAP)
             */
            mscratch_write((&mut KERNEL_TRAP_FRAME[self.cpu] as *mut TrapFrame) as usize);

            asm!("csrw  mepc, {0}", in(reg) next_pc);
            /*
             * task.resume is the function we use to resume next task into U mode
             * stval is no longer relevant at this point and can be treated as another scratch
             * register
             */
            asm!("csrw  stval, {0}", in(reg) task_s1val);
            asm!("mv   s1, {0}", in(reg) tasktrap_addr );

            asm!("ld      x0,  0   * 8(s1)");
            asm!("ld      x1,  1   * 8(s1)");
            asm!("ld      x2,  2   * 8(s1)");
            asm!("ld      x3,  3   * 8(s1)");
            asm!("ld      x4,  4   * 8(s1)");
            asm!("ld      x5,  5   * 8(s1)");
            asm!("ld      x6,  6   * 8(s1)");
            asm!("ld      x7,  7   * 8(s1)");
            asm!("ld      x8,  8   * 8(s1)");
            asm!("ld      x10, 10  * 8(s1)");
            asm!("ld      x11, 11  * 8(s1)");
            asm!("ld      x12, 12  * 8(s1)");
            asm!("ld      x13, 13  * 8(s1)");
            asm!("ld      x14, 14  * 8(s1)");
            asm!("ld      x15, 15  * 8(s1)");
            asm!("ld      x16, 16  * 8(s1)");
            asm!("ld      x17, 17  * 8(s1)");
            asm!("ld      x18, 18  * 8(s1)");
            asm!("ld      x19, 19  * 8(s1)");
            asm!("ld      x20, 20  * 8(s1)");
            asm!("ld      x21, 21  * 8(s1)");
            asm!("ld      x22, 22  * 8(s1)");
            asm!("ld      x23, 23  * 8(s1)");
            asm!("ld      x24, 24  * 8(s1)");
            asm!("ld      x25, 25  * 8(s1)");
            asm!("ld      x26, 26  * 8(s1)");
            asm!("ld      x27, 27  * 8(s1)");
            asm!("ld      x28, 28  * 8(s1)");
            asm!("ld      x29, 29  * 8(s1)");
            asm!("ld      x30, 30  * 8(s1)");
            asm!("ld      x31, 31  * 8(s1)");
            //load back satp value
            asm!("ld      x9, 31 * 8(s1)");
            asm!("csrw      satp, s1");

            asm!("csrrw   s1, stval, s1");

            asm!("sfence.vma");
            asm!("mret");
        }
    }

    pub fn resume_from_S(&mut self) {
        let next_pc = self.pc;
        unsafe {
            match self.typ {
                task_typ::KERN_TASK => {
                    sstatus::set_spp(sstatus::SPP::Supervisor);
                }
                task_typ::USER_TASK => {
                    sstatus::set_spp(sstatus::SPP::User);
                }
            }
            let tasktrap_addr = &self.trap_frame as *const TrapFrame;
            let task_s1val = KERNEL_TRAP_FRAME[self.cpu].regs[8];

            sscratch_write(tasktrap_addr as usize);

            asm!("csrw  sepc, {0}", in(reg) next_pc);
            /*
             * task.resume is the function we use to resume next task into U mode
             * stval is no longer relevant at this point and can be treated as another scratch
             * register
             */
            asm!("csrw  stval, {0}", in(reg) task_s1val);
            asm!("mv   s1, {0}", in(reg) tasktrap_addr );

            asm!("ld      x0,  0   * 8(s1)");
            asm!("ld      x1,  1   * 8(s1)");
            asm!("ld      x2,  2   * 8(s1)");
            asm!("ld      x3,  3   * 8(s1)");
            asm!("ld      x4,  4   * 8(s1)");
            asm!("ld      x5,  5   * 8(s1)");
            asm!("ld      x6,  6   * 8(s1)");
            asm!("ld      x7,  7   * 8(s1)");
            asm!("ld      x8,  8   * 8(s1)");
            asm!("ld      x10, 10  * 8(s1)");
            asm!("ld      x11, 11  * 8(s1)");
            asm!("ld      x12, 12  * 8(s1)");
            asm!("ld      x13, 13  * 8(s1)");
            asm!("ld      x14, 14  * 8(s1)");
            asm!("ld      x15, 15  * 8(s1)");
            asm!("ld      x16, 16  * 8(s1)");
            asm!("ld      x17, 17  * 8(s1)");
            asm!("ld      x18, 18  * 8(s1)");
            asm!("ld      x19, 19  * 8(s1)");
            asm!("ld      x20, 20  * 8(s1)");
            asm!("ld      x21, 21  * 8(s1)");
            asm!("ld      x22, 22  * 8(s1)");
            asm!("ld      x23, 23  * 8(s1)");
            asm!("ld      x24, 24  * 8(s1)");
            asm!("ld      x25, 25  * 8(s1)");
            asm!("ld      x26, 26  * 8(s1)");
            asm!("ld      x27, 27  * 8(s1)");
            asm!("ld      x28, 28  * 8(s1)");
            asm!("ld      x29, 29  * 8(s1)");
            asm!("ld      x30, 30  * 8(s1)");
            asm!("ld      x31, 31  * 8(s1)");
            //load back satp value
            asm!("ld      x9, 64 * 8(s1)");
            asm!("csrw      satp, s1");

            asm!("csrrw   s1, stval, s1");

            asm!("sfence.vma");

            asm!("sret");
        }
    }
}

pub struct task_pool {
    POOL: [Option<Box<Vec<task_struct>>>; MAX_HARTS],
    onlline_cpu_cnt: usize,
    current_task: [Option<usize>; MAX_HARTS],
    next_task: [Option<usize>; MAX_HARTS],
}

impl task_pool {
    pub const fn new() -> Self {
        Self {
            POOL: [None, None, None, None],
            onlline_cpu_cnt: MAX_HARTS,
            current_task: [None, None, None, None],
            next_task: [None, None, None, None],
        }
    }

    pub fn init(&mut self, cpucnt: usize) {
        for cpuid in 0..cpucnt {
            for (i, mut e) in self.POOL.iter_mut().enumerate() {
                *e = Some(Box::new(Vec::new()));
            }
            self.next_task[cpuid] = Some(0);
            self.current_task[cpuid] = Some(0);
        }
    }

    /*
     * TODO:
     * generate_next() should also check if the next task is schedulable
     * This is not a problem for ktask, but since we are going to use same
     * scheduler as future userspace program, we still need to implement this
     */
    fn generate_next(&mut self, cpuid: usize) -> Result<(), KError> {
        let taskq = self.POOL[cpuid]
            .as_ref()
            .expect("Failed to take reference of task queue");
        match self.next_task[cpuid] {
            Some(ref mut next_ent) => {
                let tmp = *next_ent;
                *next_ent = (tmp + 1) % taskq.len();
            }
            None => {
                return Err(new_kerror!(KErrorType::EFAULT));
            }
        }

        Ok(())
    }

    pub fn save_from_ktrapframe(&mut self, cpuid: usize) -> Result<(), KError> {
        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref mut taskvec) => {
                    taskvec[cur_taskidx].save();
                    return Ok(());
                }
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }

    pub fn set_current_state(&mut self, cpuid: usize, new_state: task_state) -> Result<(), KError> {
        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref mut taskvec) => {
                    taskvec[cur_taskidx].set_state(new_state);
                    return Ok(());
                }
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }

    pub fn set_currentPC(&mut self, cpuid: usize, newpc: usize) -> Result<(), KError> {
        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref mut taskvec) => {
                    taskvec[cur_taskidx].set_pc(newpc);
                    return Ok(());
                }
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }

    pub fn append_task(&mut self, new_task: task_struct, cpuid: usize) -> Result<(), KError> {
        if let Some(boxvec) = &mut self.POOL[cpuid] {
            boxvec.push(new_task);
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }

        Ok(())
    }

    pub fn sched(&mut self, cpuid: usize) -> Result<(), KError> {
        self.current_task[cpuid] = self.next_task[cpuid];

        self.generate_next(cpuid)?;

        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref mut taskvec) => {
                    if let Mode::Machine = get_cpu_mode(cpuid) {
                        taskvec[cur_taskidx].resume_from_M();
                        Ok(())
                    } else {
                        taskvec[cur_taskidx].resume_from_S();
                        Ok(())
                    }
                }
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }
}
