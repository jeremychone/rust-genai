//! API Documentation:     https://platform.openai.com/docs/api-reference/chat
//! Model Names:           https://platform.openai.com/docs/models
//! Pricing:               https://platform.openai.com/docs/pricing/ (user: https://openai.com/api/pricing/)

// region:    --- Modules

mod adapter_impl;
mod embed;
mod openai_custom;
mod streamer;

pub use adapter_impl::*;
pub use openai_custom::*;
pub use streamer::*;

// endregion: --- Modules
