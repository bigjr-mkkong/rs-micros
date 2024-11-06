use crate::asm;
use crate::cpu::{get_cpu_mode, mepc_read, mepc_write, sscratch_write, which_cpu, Mode, TrapFrame};
use crate::KERNEL_TRAP_FRAME;
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
            self.pc = mepc_read() + 4;
            self.trap_frame = KERNEL_TRAP_FRAME[cur_cpu];
        }
    }

    pub fn resume(&mut self) {
        self.resume_from_M();
    }

    fn resume_from_M(&mut self) {
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

            asm!("mret");
        }
    }
}
