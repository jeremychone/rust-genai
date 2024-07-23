// region:    --- Modules

mod support;
mod webc;

mod client;
mod common;
mod error;

// -- Flatten
pub use client::*;
pub use common::*;
pub use error::{Error, Result};

// -- Public Modules
pub mod adapter;
pub mod chat;
pub mod resolver;

// endregion: --- Modules
