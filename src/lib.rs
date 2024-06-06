// region:    --- Modules

mod common;
mod error;

pub use common::*;
pub use error::{Error, Result};

pub(crate) mod utils;
pub(crate) mod webc;

pub mod adapter;
pub mod chat;
pub mod client;
pub mod resolver;

// endregion: --- Modules
