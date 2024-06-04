//! Note: In this module, the sub-modules are
//!       for code organization and all constructs are flatten out.

// region:    --- Modules

mod chat_req;
mod chat_res;
mod legacy_client;
mod legacy_client_config;
mod tool;

// -- Flatten
pub use chat_req::*;
pub use chat_res::*;
pub use legacy_client::*;
pub use legacy_client_config::*;
pub use tool::*;

// endregion: --- Modules
