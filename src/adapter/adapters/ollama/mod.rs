//! NOTE:           Currently, GenAI uses the OpenAI compatibility layer, except for listing models.
//! OPENAI API DOC: <https://platform.openai.com/docs/api-reference/chat>
//! OLLAMA API DOC: <https://github.com/ollama/ollama/blob/main/docs/api.md>
//!  OLLAMA Models: <https://ollama.com/library>

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules
