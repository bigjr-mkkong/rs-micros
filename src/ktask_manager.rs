use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::proc::{task_pool, task_state, task_struct};
use crate::KTHREAD_POOL;

impl task_pool {
    pub fn spawn(&mut self, func: usize, cpuid: usize) -> Result<(), KError> {
        let mut pcb_newtask = task_struct::new();
        pcb_newtask.init(func)?;
        self.append_task(pcb_newtask, cpuid)?;
        Ok(())
    }

    pub fn join_all(&mut self, cpuid: usize) -> Result<usize, KError> {
        self.sched(cpuid);
        Ok(0)
    }
}

impl task_struct {
    pub fn exit(&mut self) {
        self.set_state(task_state::Zombie);
    }
}
