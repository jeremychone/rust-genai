//! API Documentation:     https://platform.openai.com/docs/api-reference/chat
//! Model Names:           https://platform.openai.com/docs/models
//! Pricing:               https://openai.com/api/pricing/

// region:    --- Modules

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules
