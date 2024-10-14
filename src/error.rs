use core::fmt;
use crate::{M_UART, S_UART};

#[macro_export]
macro_rules! new_kerror{
    ($er_type:expr) => {
        KError::new($er_type, file!(), core::module_path!(), line!())
    }
}

pub enum KErrorType{
    EFAULT,
    EINVAL,
    ENOMEM,
    ENOSYS,
}

pub struct KError{
    er_type: KErrorType,
    er_fname: &'static str,
    er_func: &'static str,
    er_line: u32,
}

impl KError{
    pub fn new(_er_type: KErrorType, 
                _er_fname: &'static str,
                _er_func: &'static str,
                _er_line: u32) ->Self{
        KError{
            er_type: _er_type,
            er_fname: _er_fname,
            er_func: _er_func,
            er_line: _er_line
        }
    }
}

impl fmt::Display for KError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let er_str = match self.er_type{
            KErrorType::EFAULT => "EFAULT",
            KErrorType::EINVAL => "EINVAL",
            KErrorType::ENOMEM => "ENOMEM",
            KErrorType::ENOSYS => "ENOSYS"
        };

        write!(
            f,
            "{} in {}: {}: at line {}",
            er_str, self.er_fname, self.er_func, self.er_line
        )
    }
}

