use crate::cpu::which_cpu;
use crate::new_kerror;
use crate::SECALL_FRAME;
use crate::{KError, KErrorType};
use riscv::asm::ecall;

#[derive(Clone, Copy)]
pub enum S2Mop {
    YIELD,
    EXIT,
    BLOCK,
    UNBLOCK,
    CLI,
    STI,
    UNDEF,
}

#[derive(Clone, Copy)]
pub enum U2Sop {
    UNDEF,
    SEND,
    RECV,
    SEND_RECV,
}

#[derive(Clone, Copy)]
pub struct ecall_args {
    sbiop: S2Mop,
    syscallop: U2Sop,
    args: [usize; 5],
    ret: usize,
}

impl ecall_args {
    pub const fn new() -> Self {
        ecall_args {
            sbiop: S2Mop::UNDEF,
            syscallop: U2Sop::UNDEF,
            args: [0 as usize; 5],
            ret: 0 as usize,
        }
    }

    pub fn get_opcode(&self) -> S2Mop {
        self.sbiop
    }

    pub fn set_opcode(&mut self, new_op: S2Mop) {
        self.sbiop = new_op;
    }

    pub fn set_args(&mut self, args: &[usize; 5]) {
        self.args = *args;
    }

    pub fn get_args(&self) -> &[usize; 5] {
        &self.args
    }

    pub fn get_ret(&self) -> usize {
        self.ret
    }

    pub fn set_ret(&mut self, new_ret: usize) {
        self.ret = new_ret;
    }
}

pub fn trapping(opcode: S2Mop, args: Option<&[usize; 5]>) -> Result<usize, KError> {
    let cur_cpu = which_cpu();
    let mut ret_val: usize;
    match args {
        Some(arg_ref) => unsafe {
            SECALL_FRAME[cur_cpu].set_opcode(opcode);
            SECALL_FRAME[cur_cpu].set_args(arg_ref);
            SECALL_FRAME[cur_cpu].set_ret(0);
            ecall();
            ret_val = SECALL_FRAME[cur_cpu].get_ret()
        },
        None => unsafe {
            SECALL_FRAME[cur_cpu].set_opcode(opcode);
            SECALL_FRAME[cur_cpu].set_ret(0);
            ecall();
            ret_val = SECALL_FRAME[cur_cpu].get_ret()
        },
    }

    Ok(ret_val)
}
