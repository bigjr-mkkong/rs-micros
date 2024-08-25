use core::fmt;

pub enum KErrorType{
    EFAULT,
    EINVAL,
    ENOMEM,
}

pub struct KError{
    er_type: KErrorType,
    er_fname: &'static str,
    er_func: &'static str,
    er_line: u32,
}

impl KError{
    pub fn new(_er_type: KErrorType) ->Self{
        KError{
            er_type: _er_type,
            er_fname: file!(),
            er_func: core::any::type_name::<fn()>(),
            er_line: line!()
        }
    }
}

impl fmt::Display for KError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let er_str = match self.er_type{
            KErrorType::EFAULT => "EFAULT",
            KErrorType::EINVAL => "EINVAL",
            KErrorType::ENOMEM => "ENOMEM"
        };

        write!(
            f,
            "{} in function {} at {}: line {}",
            er_str, self.er_func, self.er_fname, self.er_line
        )
    }
}

