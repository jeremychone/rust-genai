//! API Documentation:     https://docs.anthropic.com/en/api/messages
//! Tool Documentation:    https://docs.anthropic.com/en/docs/build-with-claude/tool-use
//! Model Names:           https://docs.anthropic.com/en/docs/models-overview
//! Pricing:               https://www.anthropic.com/pricing#anthropic-api

// region:    --- Modules

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules
