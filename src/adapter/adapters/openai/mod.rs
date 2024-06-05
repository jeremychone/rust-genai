//! API DOC: https://platform.openai.com/docs/api-reference/chat

// region:    --- Modules

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules
