// region:    --- Modules

mod error;
mod types;

pub use error::{Error, Result};
pub use types::*;

pub(crate) mod utils;
pub(crate) mod webc;

pub mod adapter;
pub mod client;

// -- TO DEPRECATE
mod providers;
#[allow(unused_imports)]
pub use providers::*;

// endregion: --- Modules
