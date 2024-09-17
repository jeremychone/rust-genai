//! genai library - Any AI provider AI client library.
//! See [examples/c00-readme.rs](./examples/c00-readme.rs)

// region:    --- Modules

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
pub mod webc;

// endregion: --- Modules
