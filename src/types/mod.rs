//! Note: In this module, the sub-modules are
//!       for code organization and all constructs are flatten out.

// region:    --- Modules

mod chat;
mod client;
mod tool;

// -- Flatten
pub use chat::*;
pub use client::*;
pub use tool::*;

// endregion: --- Modules

#[derive(Debug)]
pub enum OutFormat {
	Text,
	Json,
}
