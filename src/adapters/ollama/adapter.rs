use crate::Client;
use ollama_rs::Ollama;
use std::sync::Arc;

// region:    --- Adapter

#[derive(Debug, Default)]
pub struct OllamaAdapter {
	pub(in crate::adapters::ollama) conn: Arc<Ollama>,
}

// Constructors
impl OllamaAdapter {
	/// Returns the client trait implementation.
	pub fn default_client() -> impl Client {
		OllamaAdapter::default()
	}

	/// Returns the client trait implementation.
	pub fn new_client(host: impl Into<String>, port: u16) -> impl Client {
		OllamaAdapter::new(host, port)
	}

	/// Create a new OllamaAdapter with host and port
	/// - host: e.g., `http://127.0.0.1` (default)
	/// - poty: e.g., `11434` (default)
	pub fn new(host: impl Into<String>, port: u16) -> Self {
		let conn = Arc::new(Ollama::new(host.into(), port));
		Self { conn }
	}
}

// endregion: --- Adapter
