//! API DOC:     https://ai.google.dev/api/rest/v1beta/models/generateContent
//! MODEL NAMES: https://ai.google.dev/gemini-api/docs/models/gemini
//! PRICING:     https://ai.google.dev/pricing

// region:    --- Modules

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
pub use streamer::*;

// endregion: --- Modules
