//! API Documentation:     <https://docs.anthropic.com/en/api/messages>
//! Tool Documentation:    <https://docs.anthropic.com/en/docs/build-with-claude/tool-use>
//! Effort Documentation:  <https://platform.claude.com/docs/en/build-with-claude/effort>
//! Model Names:           <https://docs.anthropic.com/en/docs/models-overview>
//! Pricing:               <https://www.anthropic.com/pricing#anthropic-api>

// region:    --- Modules

mod ant_model;
mod ant_reasoning;
mod ant_reasoning_legacy; // unused, until confident nothing to loss from older way

mod adapter_impl;
mod adapter_shared;
mod streamer;

pub use adapter_impl::*;
pub(in crate::adapter::adapters) use adapter_shared::*;
pub use streamer::*;

// endregion: --- Modules
