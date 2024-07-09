//! OPENAI API DOC: https://platform.openai.com/docs/api-reference/chat
//!           NOTE: For now, genai uses the OpenAI compatibility layer, except for list models.
//! OLLAMA API DOC: https://github.com/ollama/ollama/blob/main/docs/api.md

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules
