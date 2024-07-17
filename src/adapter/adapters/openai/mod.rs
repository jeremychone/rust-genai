//! API DOC:     https://platform.openai.com/docs/api-reference/chat
//! MODEL NAMES: https://platform.openai.com/docs/models
//! PRICING:     https://openai.com/api/pricing/

// region:    --- Modules

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules
