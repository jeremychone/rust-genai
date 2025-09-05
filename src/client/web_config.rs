use std::time::Duration;

/// Reqwest client configuration.
#[derive(Debug, Default, Clone)]
pub struct WebConfig {
	pub timeout: Option<Duration>,
	pub connect_timeout: Option<Duration>,
	pub read_timeout: Option<Duration>,
	pub default_headers: Option<reqwest::header::HeaderMap>,
	pub proxy: Option<reqwest::Proxy>,
}

impl WebConfig {
	/// Sets the per-request timeout.
	pub fn with_timeout(mut self, timeout: Duration) -> Self {
		self.timeout = Some(timeout);
		self
	}

	/// Sets the connect timeout.
	pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
		self.connect_timeout = Some(timeout);
		self
	}

	/// Sets default headers.
	pub fn with_default_headers(mut self, headers: reqwest::header::HeaderMap) -> Self {
		self.default_headers = Some(headers);
		self
	}

	/// Sets the proxy.
	pub fn with_proxy(mut self, proxy: reqwest::Proxy) -> Self {
		self.proxy = Some(proxy);
		self
	}

	/// Sets the HTTP proxy from a URL.
	pub fn with_proxy_url(mut self, proxy_url: &str) -> Result<Self, reqwest::Error> {
		let proxy = reqwest::Proxy::http(proxy_url)?;
		self.proxy = Some(proxy);
		Ok(self)
	}

	/// Sets the HTTPS proxy from a URL.
	pub fn with_https_proxy_url(mut self, proxy_url: &str) -> Result<Self, reqwest::Error> {
		let proxy = reqwest::Proxy::https(proxy_url)?;
		self.proxy = Some(proxy);
		Ok(self)
	}

	/// Sets the proxy for all schemes from a URL.
	pub fn with_all_proxy_url(mut self, proxy_url: &str) -> Result<Self, reqwest::Error> {
		let proxy = reqwest::Proxy::all(proxy_url)?;
		self.proxy = Some(proxy);
		Ok(self)
	}

	/// Applies this config to a reqwest::ClientBuilder.
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
