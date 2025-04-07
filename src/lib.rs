//! `genai` library - A client library for any AI provider.
//! See [examples/c00-readme.rs](./examples/c00-readme.rs)

// IMPORTANT: This branc merge the support for Aync
//

#![feature(async_fn_traits)]

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
