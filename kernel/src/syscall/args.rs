use interface::{SysResult, SysError};

use crate::interrupt::Registers;
use crate::object::Handle;

pub trait UserArg: Sized {
    fn from_reg(reg: u64) -> SysResult<Self>;
}

impl UserArg for u64 {
    fn from_reg(reg: u64) -> SysResult<u64> {
        Ok(reg)
    }
}

impl UserArg for Handle {
    fn from_reg(reg: u64) -> SysResult<Handle> {
        Handle::from_u64(reg).ok_or(SysError::BadHandle)
    }
}

impl UserArg for Option<Handle> {
    fn from_reg(reg: u64) -> SysResult<Option<Handle>> {
        Ok(Handle::from_u64(reg))
    }
}

// pub trait CallRegs {
//     type Ret;
//     fn call_regs(&self, regs: &mut Registers) -> Self::Ret;
//     // fn call_regs(&self, regs: &mut Registers) -> Self::Ret;
// }

// macro_rules! impl_call_regs {
//     ($($tys:tt => ($($args:ident ,)*) ;)*) => {
//         $(
//             impl<F, Ret> CallRegs for F
//                 where F: Fn$tys -> Ret,
//             {
//                 type Ret = Ret;

//                 fn call_regs(&self, regs: &mut Registers) -> Ret {
//                     self($( regs.$args ,)*)
//                 }
//             }
//         )*
//     }
// }

// impl_call_regs! {
//     (u64)                => (rdi,);
//     (u64, u64)           => (rdi, rsi,);
//     (u64, u64, u64)      => (rdi, rsi, rdx,);
//     (u64, u64, u64, u64) => (rdi, rsi, rdx, rcx,);
// }

// pub fn wrap_syscall(F: )
