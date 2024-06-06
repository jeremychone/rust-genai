use crate::client::ClientConfig;
use crate::webc::WebClient;
use crate::Result;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Client {
	inner: Arc<ClientInner>,
}

impl Client {
	pub fn new() -> Result<Self> {
		let web_client = WebClient::new();
		let inner = ClientInner {
			web_client,
			config: Default::default(),
		};
		Ok(Self { inner: Arc::new(inner) })
	}
}

/// Client only functions
impl Client {
	pub(crate) fn web_client(&self) -> &WebClient {
		&self.inner.web_client
	}
	#[allow(unused)]
	pub(crate) fn config(&self) -> &ClientConfig {
		&self.inner.config
	}
}

#[derive(Debug)]
struct ClientInner {
	pub web_client: WebClient,

	#[allow(unused)] // for now, we do not use it
	pub config: ClientConfig,
}
