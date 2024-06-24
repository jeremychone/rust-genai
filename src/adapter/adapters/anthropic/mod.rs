//!     API DOC: https://docs.anthropic.com/en/api/messages
//! MODEL NAMES: https://docs.anthropic.com/en/docs/models-overview
//!     PRICING: https://www.anthropic.com/pricing#anthropic-api

// region:    --- Modules

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules
