use crate::Client;
use ollama_rs::Ollama;
use std::sync::Arc;

// region:    --- Provider

/// OllamaRs client provider
#[derive(Debug, Default)]
pub struct OllamaProvider {
	pub(in crate::providers::ollama) conn: Arc<Ollama>,
}

// Constructors
impl OllamaProvider {
	/// Returns the client trait implementation.
	pub fn default_client() -> impl Client {
		OllamaProvider::default()
	}

	/// Returns the client trait implementation.
	pub fn new_client(host: impl Into<String>, port: u16) -> impl Client {
		OllamaProvider::new(host, port)
	}

	/// Create a new OllamaProvider with host and port
	/// - host: e.g., `http://127.0.0.1` (default)
	/// - poty: e.g., `11434` (default)
	pub fn new(host: impl Into<String>, port: u16) -> Self {
		let conn = Arc::new(Ollama::new(host.into(), port));
		Self { conn }
	}
}

// endregion: --- Provider
