use crate::{Client, ClientConfig, Result};
use ollama_rs::Ollama;
use std::sync::Arc;

// region:    --- Provider

/// OllamaRs client provider
#[derive(Debug, Default, Clone)]
pub struct OllamaProvider {
	inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
	conn: Ollama,
	#[allow(unused)] // for now, we do not use it
	config: Option<ClientConfig>,
}

impl OllamaProvider {
	pub fn conn(&self) -> &Ollama {
		&self.inner.conn
	}
	pub fn config(&self) -> Option<&ClientConfig> {
		self.inner.config.as_ref()
	}
}

// Constructors
impl OllamaProvider {
	/// Returns the client trait implementation.
	pub fn default_client() -> impl Client {
		OllamaProvider::default()
	}

	/// Returns the client trait implementation.
	pub fn new_client(config: ClientConfig) -> Result<impl Client> {
		OllamaProvider::new_provider(config)
	}

	/// Create a new OllamaProvider with host and port in the ClientConfig
	/// Note: other properties will be ignored as Ollama client does not support them.
	pub fn new_provider(config: ClientConfig) -> Result<Self> {
		// for now, only host/port
		let conn = if let Some(endpoint) = config.endpoint.as_ref() {
			let host = endpoint.host.as_deref().unwrap_or("127.0.0.1");
			let port = endpoint.port.unwrap_or(11434);
			Ollama::new(host.to_string(), port)
		} else {
			Ollama::default()
		};
		let inner = Inner {
			config: Some(config),
			conn,
		};

		Ok(Self { inner: inner.into() })
	}
}

// endregion: --- Provider
