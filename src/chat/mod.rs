//! Note: In this module, the sub-modules are
//!       for code organization and all constructs are flatten out.

// region:    --- Modules

mod chat_options;
mod chat_req;
mod chat_res;
mod chat_stream;
mod tool;

// -- Flatten
pub use chat_options::*;
pub use chat_req::*;
pub use chat_res::*;
pub use chat_stream::*;
pub use tool::*;

// endregion: --- Modules
