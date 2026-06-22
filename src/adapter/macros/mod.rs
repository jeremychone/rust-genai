//! Macros for the crate::adapter module and sub modules

mod adapter_kind_macros;
mod dispatcher_macros;

pub(in crate::adapter) use adapter_kind_macros::*;
pub(in crate::adapter) use dispatcher_macros::*;
