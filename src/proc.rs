use crate::asm;
use crate::cpu::{
    busy_delay, get_cpu_mode, make_satp, mepc_read, mepc_write, satp_write, sscratch_write,
    which_cpu, Mode, SATP_mode, TrapFrame, MAX_HARTS,
};
use crate::ecall;
use crate::ecall::S2Mop;
use crate::error::{KError, KErrorType};
use crate::kmem::{get_ksatp, get_page_table};
use crate::page::PAGE_SIZE;
use crate::vm::{ident_range_map, mem_map, EntryBits, PageEntry, PageTable};
use crate::zone::{kfree_page, kmalloc_page, zone_type};
use crate::KERNEL_TRAP_FRAME;
use crate::{M_UART, S_UART};
use riscv::register::{mstatus, sstatus};
enum task_state {
    Ready,
    Running,
    Block,
    Zombie,
    Dead,
}

enum task_typ {
    KERN_TASK,
    USER_TASK,
}

/*
 * cpu need to keep same as current hartid
 * We can have per-cpu schedule queue, and each task in a single queue need to have same cpu value
 */
pub struct task_struct {
    trap_frame: TrapFrame,
    state: task_state,
    pc: usize,
    cpu: usize,
    pid: usize,
    typ: task_typ,
}

impl task_struct {
    pub const fn new() -> Self {
        Self {
            trap_frame: TrapFrame::new(),
            state: task_state::Ready,
            pc: 0 as usize,
            cpu: 0 as usize,
            pid: 0 as usize,
            typ: task_typ::KERN_TASK,
        }
    }

    pub fn init(&mut self) -> Result<usize, KError> {
        // let mut new_pcb = Self{
        //     trap_frame: TrapFrame::new(),
        //     state: task_state::Ready,
        //     pc: 0 as usize,
        //     cpu: which_cpu(),
        //     pid: 0,
        //     typ: task_typ::KERN_TASK
        // };
        self.cpu = which_cpu();
        self.trap_frame.cpuid = self.cpu;
        if let task_typ::KERN_TASK = self.typ {
            self.trap_frame.satp = get_ksatp() as usize;
            let pageroot_ptr = get_page_table();
            let mut pageroot = unsafe { pageroot_ptr.as_mut().unwrap() };
            //initialize kernel task
            unsafe {
                self.pc = KHello as usize;
                self.pid = 0;

                let kt_stack = kmalloc_page(zone_type::ZONE_NORMAL, 1)?.add(PAGE_SIZE * 1);
                ident_range_map(
                    pageroot,
                    kt_stack.sub(1 * PAGE_SIZE) as usize,
                    kt_stack.sub(1 * PAGE_SIZE) as usize,
                    EntryBits::ReadWrite.val(),
                );
                self.trap_frame.regs[2] = kt_stack as usize;
            }
        } else {
            //initialize user task
            let pg_root_ptr = kmalloc_page(zone_type::ZONE_NORMAL, 1)? as *mut PageTable;
            let pg_root = unsafe { pg_root_ptr.as_mut().unwrap() };

            let satp_root = pg_root_ptr as usize;
            self.trap_frame.satp = make_satp(SATP_mode::Sv39, 0, satp_root);

            unsafe {
                self.pc = KHello as usize;

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
            sscratch_write((&KERNEL_TRAP_FRAME[cur_cpu] as *const TrapFrame) as usize);
            self.trap_frame.save_from(&KERNEL_TRAP_FRAME[cur_cpu]);
        }
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
            asm!("ld      x9, 31 * 8(s1)");
            asm!("csrw      satp, s1");

            asm!("csrrw   s1, stval, s1");

            asm!("sfence.vma");

            asm!("sret");
        }
    }
}

#[no_mangle]
extern "C" fn KHello() {
    println!("Hello from KHello at CPU{}", which_cpu());
    // ecall::trapping(S2Mop::TEST, &[0xdeadbeef, 0xbadc0de, 0xfea123, 0, 0]);
    // println!("Returned from ECALL");
    loop {
        let _ = busy_delay(1);
        unsafe {
            asm!("nop");
        }
    }
}
