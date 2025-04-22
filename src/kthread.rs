use crate::alloc::boxed::Box;
use crate::alloc::vec::Vec;
use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, mscratch_write, satp_write,
    sscratch_write, which_cpu, M_cli, M_sti, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall;
use crate::ecall::S2Mop;
use crate::error::{KError, KErrorType};
use crate::kmem::{get_ksatp, get_page_table};
use crate::ksemaphore::kt_semaphore;
use crate::ktask::{ktask_extint, ktask_fallback};
use crate::lock::{spin_mutex, S_lock};
use crate::new_kerror;
use crate::page::PAGE_SIZE;
use crate::vm::{ident_range_map, mem_map, range_unmap, EntryBits, PageEntry, PageTable};
use crate::zone::{kfree_page, kmalloc_page, zone_type};
use crate::IRQ_BUFFER;
use crate::KERNEL_TRAP_FRAME;
use crate::KTHREAD_POOL;
use crate::{M_UART, S_UART};
use cbitmap::bitmap::*;
use core::cell::UnsafeCell;
use core::hash::*;
use riscv::register::{mstatus, sstatus};

pub const MAX_KTHREADS: usize = 256;
pub const INVAL_KTHREADS_PID: usize = MAX_KTHREADS + 10;

#[derive(Clone, Copy)]
pub enum task_state {
    Ready,
    Running,
    Block,
    Zombie,
    Dead,
}

//TODO Remove "_TASK"
#[derive(Clone, Copy)]
pub enum task_typ {
    KERN_TASK,
    USER_TASK,
}

#[derive(Clone, Copy)]
pub enum task_flag {
    CRITICAL,
    NORMAL,
}

const MAX_KTASK: usize = 100;
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
    flag: task_flag,
    life_id: usize,
}

impl Drop for task_struct {
    fn drop(&mut self) {
        let mut pageroot = unsafe { get_page_table().as_mut().unwrap() };

        let kt_stack_begin: *mut u8 = (self.stack_base - KTASK_STACK_SZ) as *mut u8;
        kfree_page(zone_type::ZONE_NORMAL, kt_stack_begin);
        range_unmap(pageroot, kt_stack_begin as usize, self.stack_base);

        let exp_stack_begin: *mut u8 = (self.exp_stack_base - KTASK_EXPSTACK_SZ) as *mut u8;
        kfree_page(zone_type::ZONE_NORMAL, exp_stack_begin);
        range_unmap(pageroot, exp_stack_begin as usize, self.exp_stack_base);
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
            flag: task_flag::NORMAL,
            life_id: 0,
        }
    }

    pub fn get_state(&self) -> task_state {
        self.state
    }

    pub fn set_state(&mut self, new_state: task_state) {
        self.state = new_state;
    }

    pub fn get_lifeid(&self) -> usize {
        self.life_id
    }

    pub fn set_lifeid(&mut self, new_lifeid: usize) {
        self.life_id = new_lifeid;
    }
    pub fn init(&mut self, func: usize, new_flag: task_flag) -> Result<usize, KError> {
        self.cpu = which_cpu();
        self.trap_frame.cpuid = self.cpu;
        self.state = task_state::Ready;
        self.flag = new_flag;
        if let task_typ::KERN_TASK = self.typ {
            self.trap_frame.satp = get_ksatp() as usize;
            let mut pageroot = unsafe { get_page_table().as_mut().unwrap() };
            self.pc = func;
            self.pid = 0;
            //initialize kernel task
            unsafe {
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
            //No user task here
            return Err(new_kerror!(KErrorType::EFAULT));
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

    pub fn resume_from_M(&self) {
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

    pub fn resume_from_S(&self) {
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

//TODO
pub struct task_pool {
    POOL: [Option<Box<Vec<task_struct>>>; MAX_HARTS],
    onlline_cpu_cnt: usize,
    current_task: [Option<usize>; MAX_HARTS],
    next_task: [Option<usize>; MAX_HARTS],
    fallback_task: [Option<Box<task_struct>>; MAX_HARTS],
    crit_task_intstate: [usize; MAX_HARTS],
    pidmap: Option<spin_mutex<Bitmap<{ (MAX_KTASK / 8) + 1 }>, S_lock>>,
    pub sems: [Option<Vec<kt_semaphore>>; MAX_HARTS],
    life_id: spin_mutex<usize, S_lock>,
}

impl task_pool {
    pub const fn new() -> Self {
        Self {
            POOL: [None, None, None, None],
            onlline_cpu_cnt: MAX_HARTS,
            current_task: [None, None, None, None],
            next_task: [None, None, None, None],
            fallback_task: [None, None, None, None],
            crit_task_intstate: [0; MAX_HARTS],
            pidmap: None,
            sems: [None, None, None, None],
            life_id: spin_mutex::<usize, S_lock>::new(1),
        }
    }

    pub fn init(&mut self, cpucnt: usize) {
        for cpuid in 0..cpucnt {
            for (i, mut e) in self.POOL.iter_mut().enumerate() {
                *e = Some(Box::new(Vec::new()));
            }
            
            for fallbacker in self.fallback_task.iter_mut() {
                let mut fallb = task_struct::new();
                fallb.init(ktask_fallback as usize, task_flag::NORMAL);
                *fallbacker = Some(Box::new(fallb));
            }
            self.next_task[cpuid] = Some(0);
            self.current_task[cpuid] = Some(0);
            self.pidmap = Some(spin_mutex::new(Bitmap::new()));
        }
    }

    fn get_new_pid(&mut self) -> usize {
        let bind = self.pidmap.as_mut().unwrap();
        let mut idmap = bind.lock();
        let newid = idmap.find_first_zero();
        match newid {
            Some(new_id) => {
                idmap.set(new_id);
                return new_id;
            }
            None => {
                panic!("Run out of task pid");
            }
        }
    }

    fn reclaim_pid(&mut self, oldpid: usize) {
        let bind = self.pidmap.as_mut().unwrap();
        let mut idmap = bind.lock();
        idmap.set(oldpid);
    }
    fn generate_next(&mut self, cpuid: usize) -> Result<(), KError> {
        let taskq = self.POOL[cpuid]
            .as_ref()
            .expect("Failed to take reference of task queue");
        match self.next_task[cpuid] {
            Some(ref mut next_ent) => 'state_check: loop {
                let tmp = *next_ent;
                let taskqlen = taskq.len();

                if taskqlen == 0 {
                    *next_ent = 0;
                    break;
                } else {
                    *next_ent = (tmp + 1) % taskq.len();
                    if let Some(ref taskvec) = self.POOL[cpuid] {
                        let next_state = taskvec[*next_ent].get_state();
                        match next_state {
                            task_state::Ready => {
                                break 'state_check;
                            }
                            task_state::Running => {
                                break 'state_check;
                            }
                            _ => {}
                        }
                    } else {
                        return Err(new_kerror!(KErrorType::EFAULT));
                    }
                }
            },
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

    pub fn get_crit_task_mie(&self) -> &[usize; MAX_HARTS] {
        &self.crit_task_intstate
    }

    pub fn get_current_fg(&self, cpuid: usize) -> Result<task_flag, KError> {
        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref taskvec) => Ok(taskvec[cur_taskidx].flag),
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }

    pub fn get_current_pid(&self, cpuid: usize) -> Result<usize, KError> {
        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref taskvec) => Ok(taskvec[cur_taskidx].pid),
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }

    pub fn get_current_lifeid(&self, cpuid: usize) -> Result<usize, KError> {
        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref taskvec) => Ok(taskvec[cur_taskidx].life_id),
                None => {
                    return Err(new_kerror!(KErrorType::EINVAL));
                }
            }
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }
    }

    fn get_scheduable_cnt(&self, cpuid: usize) -> usize {
        match self.POOL[cpuid] {
            Some(ref taskvec) => {
                let mut live_cnt = 0;
                for task in taskvec.iter() {
                    match task.get_state() {
                        task_state::Ready => {
                            live_cnt += 1;
                        }
                        task_state::Running => {
                            live_cnt += 1;
                        }
                        _ => {
                            live_cnt += 0;
                        }
                    }
                }
                return live_cnt;
            }
            None => {
                return 0;
            }
        }
    }

    pub fn append_task(&mut self, mut new_task: task_struct, cpuid: usize) -> Result<(), KError> {
        new_task.pid = self.get_new_pid();

        let mut new_lifeid = self.life_id.lock();

        new_task.life_id = *new_lifeid;
        *new_lifeid += 1;
        drop(new_lifeid);

        if let Some(boxvec) = &mut self.POOL[cpuid] {
            boxvec.push(new_task);
        } else {
            return Err(new_kerror!(KErrorType::EINVAL));
        }

        Ok(())
    }

    pub fn remove_cur_task(&mut self, cpuid: usize) -> Result<(), KError> {
        let mut died_pid: usize = 0;
        if let Some(cur_taskidx) = &mut self.current_task[cpuid] {
            match &mut self.POOL[cpuid] {
                Some(ref mut boxvec) => {
                    died_pid = boxvec[*cur_taskidx].pid;
                    boxvec.swap_remove(*cur_taskidx);
                }
                None => {
                    return Err(new_kerror!(KErrorType::EFAULT));
                }
            }
            self.reclaim_pid(died_pid);
        }

        Ok(())
    }

    pub fn fallback(&mut self, cpuid: usize) -> Result<(), KError> {
        match self.fallback_task[cpuid] {
            Some(ref mut fallbacker) => {
                let current_mode = get_cpu_mode(cpuid);
                if matches!(current_mode, Mode::Machine | Mode::Machine_IRH) {
                    fallbacker.resume_from_M();
                    Ok(())
                } else {
                    fallbacker.resume_from_S();
                    Ok(())
                }
            }
            None => {
                return Err(new_kerror!(KErrorType::EINVAL));
            }
        }
    }

    pub fn sched(&mut self, cpuid: usize) -> Result<(), KError> {
        let live_cnt = self.get_scheduable_cnt(cpuid);
        self.generate_next(cpuid)?;
        self.current_task[cpuid] = self.next_task[cpuid];

        if let Some(cur_taskidx) = self.current_task[cpuid] {
            match self.POOL[cpuid] {
                Some(ref mut taskvec) => {
                    if live_cnt == 0 {
                        taskvec.clear();
                        self.current_task = [Some(0), Some(0), Some(0), Some(0)];
                        self.next_task = [Some(0), Some(0), Some(0), Some(0)];
                        return Ok(());
                    }

                    let current_mode = get_cpu_mode(cpuid);
                    if matches!(current_mode, Mode::Machine | Mode::Machine_IRH) {
                        if let task_flag::CRITICAL = taskvec[cur_taskidx].flag {
                            let prev_xie = M_cli();
                            self.crit_task_intstate[cpuid] = prev_xie;
                        }

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

    /*
     * Low efficiency, but correct
     */
    pub fn set_state_by_pid(
        &mut self,
        target_pid: usize,
        target_lifeid: usize,
        new_state: task_state,
    ) -> Result<(), KError> {
        for cpuid in 0..MAX_HARTS {
            if let Some(ref mut taskvec) = self.POOL[cpuid] {
                for task in taskvec.iter_mut() {
                    if task.pid == target_pid {
                        assert!(task.life_id == target_lifeid);
                        task.set_state(new_state);
                        return Ok(());
                    }
                }
            }
        }
        Err(new_kerror!(KErrorType::EFAULT))
    }
}

pub fn get_ktpid_lifeid(cpuid: usize) -> Result<(usize, usize), KError> {
    unsafe {
        let pid = KTHREAD_POOL.get_current_pid(cpuid)?;
        let lifeid = KTHREAD_POOL.get_current_lifeid(cpuid)?;
        Ok((pid, lifeid))
    }
}
