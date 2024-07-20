// region:    --- Modules

mod client;
mod common;
mod error;
mod support;
mod webc;

// -- Flatten
pub use client::*;
pub use common::*;
pub use error::{Error, Result};

// -- Public Modules
pub mod adapter;
pub mod chat;
pub mod resolver;
pub mod utils;

// endregion: --- Modules
