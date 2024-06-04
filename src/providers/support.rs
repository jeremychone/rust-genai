use crate::webc::WebClient;
use crate::ClientConfig;

pub(in crate::providers) trait Provider {
	/// Required - Return the provider inner
	fn provider_inner(&self) -> &ProviderInner;

	/// To be implemented by the Provider type,
	/// By default, return None
	fn default_api_key_env_name() -> Option<&'static str> {
		None
	}

	fn web_client(&self) -> &WebClient {
		&self.provider_inner().web_client
	}

	fn config(&self) -> Option<&ClientConfig> {
		self.provider_inner().config.as_ref()
	}
}

// region:    --- ProviderInner

/// The common provider inner for all native providers.
pub struct ProviderInner {
	pub(in crate::providers) web_client: WebClient,
	#[allow(unused)] // for now, we do not use it
	pub(in crate::providers) config: Option<ClientConfig>,
}

// endregion: --- ProviderInner
