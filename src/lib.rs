// region:    --- Modules

mod client;
mod common;
mod error;
mod webc;

pub use client::*;
pub use common::*;
pub use error::{Error, Result};

pub mod adapter;
pub mod chat;
pub mod resolver;
pub mod utils;

// endregion: --- Modules
