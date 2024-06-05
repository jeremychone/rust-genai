// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub(crate) mod utils;
pub(crate) mod webc;

pub mod adapter;
pub mod chat;
pub mod client;

// endregion: --- Modules
