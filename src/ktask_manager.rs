use crate::proc::{ task_struct, task_pool, task_state};
use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::KTHREAD_POOL;

impl task_pool{
    pub fn spawn(&mut self, func: usize, cpuid: usize) -> Result<(), KError>{
        let mut pcb_newtask = task_struct::new();
        pcb_newtask.init(func)?;
        self.append_task(pcb_newtask, cpuid)?;
        Ok(())
    }

    pub fn join_all() -> Result<usize, KError>{
        //join shall only unblock when all kthread has finished(Into zombie mode)
        todo!();
    }

    
}

impl task_struct{
    pub fn exit(&mut self){
        self.set_state(task_state::Zombie);
    }
}
