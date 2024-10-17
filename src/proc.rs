use crate::asm;
use riscv::register::{mstatus, sstatus};
use crate::cpu::{TrapFrame, which_cpu, get_cpu_mode, Mode};
use crate::KERNEL_TRAP_FRAME;
enum task_state{
    Ready,
    Running,
    Block,
    Zombie,
    Dead
}


/*
 * cpu need to keep same as current hartid
 * We can have per-cpu schedule queue, and each task in a single queue need to have same cpu value
 */
pub struct task_struct{
    state_frame: TrapFrame,
    state: task_state,
    stack: usize,
    pc: usize,
    pgt_root: usize,
    cpu: usize
}

impl task_struct{
    /*
     * task.save will save records from KERNL_TRAP_FRAME to specific task's trapframe
     * infoe KERNEL_TRAP_FRAME.regs are guarantee to be the cpu state before trapping
     * so we can directly copy the contents on it
     */
    pub fn save(&mut self){
        let cur_cpu = self.cpu;
        unsafe{
            let usr_record = KERNEL_TRAP_FRAME[cur_cpu].clone();
            self.state_frame = usr_record;
        }
    }

    pub fn resume(&mut self){
        if let Mode::Machine = get_cpu_mode(self.cpu){
            self.resume_from_M();
        }else{
            self.resume_from_S();
        }
    }

    fn resume_from_S(&mut self){
        let next_pc = self.pc;
        unsafe{
            sstatus::set_spp(sstatus::SPP::User);
            let tasktrap_addr = &self.state_frame as *const TrapFrame;
            let task_s1val = KERNEL_TRAP_FRAME[self.cpu].regs[8];
            asm!("csrw  sepc, {0}", in(reg) next_pc);
            /*
             * task.resume is the function we use to resume next task into U mode
             * stval is no longer relevant at this point and can be treated as another scratch
             * register
             */
            asm!("csrw  stval, {0}", in(reg) task_s1val);
            asm!("mv   s1, {0}", in(reg) tasktrap_addr );
            
            asm!("ld      x1,  0  * 8(s1)");
            asm!("ld      x2,  1  * 8(s1)");
            asm!("ld      x3,  2  * 8(s1)");
            asm!("ld      x4,  3  * 8(s1)");
            asm!("ld      x5,  4  * 8(s1)");
            asm!("ld      x6,  5  * 8(s1)");
            asm!("ld      x7,  6  * 8(s1)");
            asm!("ld      x8,  7  * 8(s1)");
            asm!("ld      x10, 9  * 8(s1)");
            asm!("ld      x11, 10 * 8(s1)");
            asm!("ld      x12, 11 * 8(s1)");
            asm!("ld      x13, 12 * 8(s1)");
            asm!("ld      x14, 13 * 8(s1)");
            asm!("ld      x15, 14 * 8(s1)");
            asm!("ld      x16, 15 * 8(s1)");
            asm!("ld      x17, 16 * 8(s1)");
            asm!("ld      x18, 17 * 8(s1)");
            asm!("ld      x19, 18 * 8(s1)");
            asm!("ld      x20, 19 * 8(s1)");
            asm!("ld      x21, 20 * 8(s1)");
            asm!("ld      x22, 21 * 8(s1)");
            asm!("ld      x23, 22 * 8(s1)");
            asm!("ld      x24, 23 * 8(s1)");
            asm!("ld      x25, 24 * 8(s1)");
            asm!("ld      x26, 25 * 8(s1)");
            asm!("ld      x27, 26 * 8(s1)");
            asm!("ld      x28, 27 * 8(s1)");
            asm!("ld      x29, 28 * 8(s1)");
            asm!("ld      x30, 29 * 8(s1)");
            asm!("ld      x31, 30 * 8(s1)");

            asm!("csrrw   s1, stval, s1");

            asm!("sret");
        }
    }

    fn resume_from_M(&mut self){
        let next_pc = self.pc;
        unsafe{
            mstatus::set_mpp(mstatus::MPP::User);
            let tasktrap_addr = &self.state_frame as *const TrapFrame;
            let task_s1val = KERNEL_TRAP_FRAME[self.cpu].regs[8];
            asm!("csrw  sepc, {0}", in(reg) next_pc);
            /*
             * task.resume is the function we use to resume next task into U mode
             * stval is no longer relevant at this point and can be treated as another scratch
             * register
             */
            asm!("csrw  stval, {0}", in(reg) task_s1val);
            asm!("mv   s1, {0}", in(reg) tasktrap_addr );
            
            asm!("ld      x1,  0  * 8(s1)");
            asm!("ld      x2,  1  * 8(s1)");
            asm!("ld      x3,  2  * 8(s1)");
            asm!("ld      x4,  3  * 8(s1)");
            asm!("ld      x5,  4  * 8(s1)");
            asm!("ld      x6,  5  * 8(s1)");
            asm!("ld      x7,  6  * 8(s1)");
            asm!("ld      x8,  7  * 8(s1)");
            asm!("ld      x10, 9  * 8(s1)");
            asm!("ld      x11, 10 * 8(s1)");
            asm!("ld      x12, 11 * 8(s1)");
            asm!("ld      x13, 12 * 8(s1)");
            asm!("ld      x14, 13 * 8(s1)");
            asm!("ld      x15, 14 * 8(s1)");
            asm!("ld      x16, 15 * 8(s1)");
            asm!("ld      x17, 16 * 8(s1)");
            asm!("ld      x18, 17 * 8(s1)");
            asm!("ld      x19, 18 * 8(s1)");
            asm!("ld      x20, 19 * 8(s1)");
            asm!("ld      x21, 20 * 8(s1)");
            asm!("ld      x22, 21 * 8(s1)");
            asm!("ld      x23, 22 * 8(s1)");
            asm!("ld      x24, 23 * 8(s1)");
            asm!("ld      x25, 24 * 8(s1)");
            asm!("ld      x26, 25 * 8(s1)");
            asm!("ld      x27, 26 * 8(s1)");
            asm!("ld      x28, 27 * 8(s1)");
            asm!("ld      x29, 28 * 8(s1)");
            asm!("ld      x30, 29 * 8(s1)");
            asm!("ld      x31, 30 * 8(s1)");

            asm!("csrrw   s1, stval, s1");

            asm!("mret");
        }
    }
}
