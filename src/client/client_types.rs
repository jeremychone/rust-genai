use crate::client::ClientConfig;
use crate::webc::WebClient;
use crate::ClientBuilder;
use std::sync::Arc;

/// genai Client for executing AI requests to any providers.
/// Built with:
/// - `ClientBuilder::default()...build()`
/// - or `Client::builder()`, which is equivalent to `ClientBuilder::default()...build()`
#[derive(Debug, Clone)]
pub struct Client {
	pub(super) inner: Arc<ClientInner>,
}

// region:    --- Client Constructors

impl Default for Client {
	fn default() -> Self {
		Client::builder().build()
	}
}

impl Client {
	/// Create a new ClientBuilder for Client
	/// This is just another way to use `ClientBuilder::default()`
	pub fn builder() -> ClientBuilder {
		ClientBuilder::default()
	}
}

// endregion: --- Client Constructors

// region:    --- Client Getters

impl Client {
	pub(crate) fn web_client(&self) -> &WebClient {
		&self.inner.web_client
	}

	pub(crate) fn config(&self) -> &ClientConfig {
		&self.inner.config
	}
}

// endregion: --- Client Getters

// region:    --- ClientInner

#[derive(Debug)]
pub(super) struct ClientInner {
	pub(super) web_client: WebClient,

	pub(super) config: ClientConfig,
}

// endregion: --- ClientInner
