//! API Documentation:     <https://opencode.ai/docs/go/>
//!
//! The OpenCode Go proxy is an OpenAI-compatible gateway for the OpenCode ecosystem.
//! Supports routing to models via the `opencode_go::` namespace prefix.

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules
