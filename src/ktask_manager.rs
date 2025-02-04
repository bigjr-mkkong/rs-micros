use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::proc::{task_pool, task_state, task_struct, task_flag};
use crate::ksemaphore::kt_semaphore;
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

impl task_pool {
    /*
     * Return value: (cpuid, idx)
     */
    pub fn sem_init(&mut self, cpuid: usize, val: i32) -> (usize, usize) {
        let new_sem = kt_semaphore::new(val);
        match self.sems[cpuid] {
            Some(ref mut semvec) => {
                semvec.push(new_sem);
                (cpuid, semvec.len()-1)
            },
            None => {
                panic!("sem_init() failed");
            }
        }
    }

    pub fn wait(&mut self, (cpuid, idx): (usize, usize)) {
        loop{
            let mut bind = self.sems[cpuid].as_mut().unwrap().get_mut(idx);
        }
    }

}
