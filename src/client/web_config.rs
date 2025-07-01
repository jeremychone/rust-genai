use std::time::Duration;

/// Configuration options for the reqwest client
#[derive(Debug, Clone)]
pub struct WebConfig {
	pub timeout: Option<Duration>,
	pub connect_timeout: Option<Duration>,
	pub read_timeout: Option<Duration>,
	pub default_headers: Option<reqwest::header::HeaderMap>,
	pub proxy: Option<reqwest::Proxy>,
}

impl Default for WebConfig {
	fn default() -> Self {
		Self {
			timeout: None,
			connect_timeout: None,
			read_timeout: None,
			default_headers: None,
			proxy: None,
		}
	}
}

impl WebConfig {
	/// Set the timeout for the reqwest client
	pub fn with_timeout(mut self, timeout: Duration) -> Self {
		self.timeout = Some(timeout);
		self
	}

	/// Set the connect timeout for the reqwest client
	pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
		self.connect_timeout = Some(timeout);
		self
	}

	/// Set default headers for the reqwest client
	pub fn with_default_headers(mut self, headers: reqwest::header::HeaderMap) -> Self {
		self.default_headers = Some(headers);
		self
	}

	/// Set proxy for the reqwest client
	pub fn with_proxy(mut self, proxy: reqwest::Proxy) -> Self {
		self.proxy = Some(proxy);
		self
	}

	/// Set proxy from URL string for the reqwest client
	pub fn with_proxy_url(mut self, proxy_url: &str) -> Result<Self, reqwest::Error> {
		let proxy = reqwest::Proxy::http(proxy_url)?;
		self.proxy = Some(proxy);
		Ok(self)
	}

	/// Set HTTPS proxy from URL string for the reqwest client
	pub fn with_https_proxy_url(mut self, proxy_url: &str) -> Result<Self, reqwest::Error> {
		let proxy = reqwest::Proxy::https(proxy_url)?;
		self.proxy = Some(proxy);
		Ok(self)
	}

	/// Set proxy for all schemes from URL string for the reqwest client
	pub fn with_all_proxy_url(mut self, proxy_url: &str) -> Result<Self, reqwest::Error> {
		let proxy = reqwest::Proxy::all(proxy_url)?;
		self.proxy = Some(proxy);
		Ok(self)
	}

	/// Apply the configuration to a reqwest ClientBuilder
	pub fn apply_to_builder(&self, mut builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
		if let Some(timeout) = self.timeout {
			builder = builder.timeout(timeout);
		}
		if let Some(connect_timeout) = self.connect_timeout {
			builder = builder.connect_timeout(connect_timeout);
		}
		if let Some(read_timeout) = self.read_timeout {
			builder = builder.read_timeout(read_timeout);
		}
		if let Some(ref headers) = self.default_headers {
			builder = builder.default_headers(headers.clone());
		}
		if let Some(ref proxy) = self.proxy {
			builder = builder.proxy(proxy.clone());
		}
		builder
	}
}
