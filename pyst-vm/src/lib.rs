#[macro_use]
extern crate log;

pub mod bytecode;
pub mod pyobject;
pub mod compile;
pub mod eval;

mod builtins;
mod exceptions;
mod frame;
mod import;
mod objbool;
mod objdict;
mod objfunction;
mod objint;
mod objlist;
mod objobject;
mod objsequence;
mod objstr;
mod objtype;
mod sysmodule;
mod vm;

pub use self::vm::VirtualMachine;