// region:    --- Modules

mod client;
mod common;
mod error;

pub use client::*;
pub use common::*;
pub use error::{Error, Result};

pub(crate) mod webc;

pub mod adapter;
pub mod chat;
pub mod resolver;
pub mod utils;

// endregion: --- Modules
