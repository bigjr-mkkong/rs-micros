use crate::cpu::MAX_HARTS;
use crate::error::{KError, KErrorType};
use crate::new_kerror;
use crate::plic::extint_name;
use crate::GETRSETR;
use ringbuffer::{AllocRingBuffer, RingBuffer};

pub const MAX_IRQ: usize = 128;

#[derive(Clone, Copy)]
pub enum int_type {
    EXTERNAL,
    INTERNAL,
    NONE,
}

#[derive(Clone, Copy)]
pub struct int_request {
    typ: int_type,
    extint_id: u32,
    cpuid: usize,
    data: Option<usize>,
}

impl int_request {
    pub const fn new() -> Self {
        Self {
            typ: int_type::NONE,
            extint_id: 0,
            cpuid: 0,
            data: None,
        }
    }

    GETRSETR!(typ, int_type);
    GETRSETR!(extint_id, u32);
    GETRSETR!(cpuid, usize);
    GETRSETR!(data, Option<usize>);
}

pub struct soft_irq_buf {
    irq_buffer: [Option<ringbuffer::AllocRingBuffer<int_request>>; MAX_HARTS],
}

impl soft_irq_buf {
    pub const fn new() -> Self {
        Self {
            irq_buffer: [None, None, None, None],
        }
    }

    pub fn init(&mut self) {
        for i in 0..MAX_HARTS {
            self.irq_buffer[i] = Some(AllocRingBuffer::new(MAX_IRQ));
        }
    }

    pub fn push_req(&mut self, req: int_request, cpuid: usize) -> Result<(), KError> {
        if let Some(ref mut irq_q) = self.irq_buffer[cpuid] {
            irq_q.push(req);
            Ok(())
        } else {
            Err(new_kerror!(KErrorType::EINVAL))
        }
    }

    pub fn peek_req(&mut self, cpuid: usize) -> Result<Option<&int_request>, KError> {
        if let Some(ref irq_q) = self.irq_buffer[cpuid] {
            let top_req = irq_q.peek();
            Ok(top_req)
        } else {
            Err(new_kerror!(KErrorType::EINVAL))
        }
    }

    pub fn dequeue_req(&mut self, cpuid: usize) -> Result<(), KError> {
        if let Some(ref mut irq_q) = self.irq_buffer[cpuid] {
            irq_q.dequeue();
            Ok(())
        } else {
            Err(new_kerror!(KErrorType::EINVAL))
        }
    }

    pub fn is_empty(&self, cpuid: usize) -> Result<bool, KError> {
        if let Some(ref irq_q) = self.irq_buffer[cpuid] {
            Ok(irq_q.is_empty())
        } else {
            Err(new_kerror!(KErrorType::EINVAL))
        }
    }

    pub fn is_full(&self, cpuid: usize) -> Result<bool, KError> {
        if let Some(ref irq_q) = self.irq_buffer[cpuid] {
            Ok(irq_q.is_full())
        } else {
            Err(new_kerror!(KErrorType::EINVAL))
        }
    }

    pub fn len(&self, cpuid: usize) -> Result<usize, KError> {
        if let Some(ref irq_q) = self.irq_buffer[cpuid] {
            Ok(irq_q.len())
        } else {
            Err(new_kerror!(KErrorType::EINVAL))
        }
    }
}
