use crate::error::{KError, KErrorType};
use crate::ksemaphore::kt_semaphore;
use crate::kthread::{task_flag, task_pool, task_state, task_struct};
use crate::new_kerror;
use crate::KTHREAD_POOL;

impl task_pool {
    pub fn spawn(&mut self, func: usize, new_flag: task_flag, cpuid: usize) -> Result<(), KError> {
        let mut pcb_newtask = task_struct::new();
        pcb_newtask.init(func, new_flag)?;
        self.append_task(pcb_newtask, cpuid)?;
        Ok(())
    }

    pub fn join_all_ktask(&mut self, cpuid: usize) -> Result<usize, KError> {
        self.sched(cpuid);
        self.fallback(cpuid);
        Ok(0)
    }
}

impl task_struct {
    pub fn exit(&mut self) {
        self.set_state(task_state::Zombie);
    }
}
