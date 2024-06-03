//! Note: In this module, the sub-modules are
//!       for code organization and all constructs are flatten out.

// region:    --- Modules

mod chat_req;
mod chat_res;
mod client;
mod client_config;
mod tool;

// -- Flatten
pub use chat_req::*;
pub use chat_res::*;
pub use client::*;
pub use client_config::*;
pub use tool::*;

// endregion: --- Modules
