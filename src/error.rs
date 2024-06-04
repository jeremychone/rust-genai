use crate::adapter::AdapterKind;
use crate::{webc, LegacyClientKind};
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
	#[from]
	Custom(String),

	ApiKeyEnvNotFound {
		env_name: String,
	},

	// -- Web
	#[from]
	Webc(webc::Error),

	// -- Adapter
	AdapterRequiresApiKey {
		adapter_kind: AdapterKind,
	},

	// -- Provider
	ProviderHasNoDefaultApiKeyEnvName,

	// -- Stream
	StreamParse(serde_json::Error),
	ReqwestEventSource(reqwest_eventsource::Error),

	// -- Utils
	#[from]
	XValue(crate::utils::x_value::Error),

	// -- Externals
	#[from]
	Io(std::io::Error), // as example

	// -- Raw AI Clients
	ProviderConnector(ProviderConnectorInfo),
}

// region:    --- Custom

impl Error {
	pub fn custom(val: impl std::fmt::Display) -> Self {
		Self::Custom(val.to_string())
	}
}

impl From<&str> for Error {
	fn from(val: &str) -> Self {
		Self::Custom(val.to_string())
	}
}

// endregion: --- Custom

// region:    --- Impl

impl Error {
	pub fn provider_connector(client_kind: LegacyClientKind, cause: impl Into<String>) -> Self {
		Self::ProviderConnector(ProviderConnectorInfo {
			client_kind,
			cause: cause.into(),
		})
	}
}

// endregion: --- Impl

/// Type that capture the provider error information
/// Each provider implements their `From<..>` for `Error` with the `ProviderConnector` variant
#[derive(Debug)]
pub struct ProviderConnectorInfo {
	pub client_kind: LegacyClientKind,
	pub cause: String,
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
