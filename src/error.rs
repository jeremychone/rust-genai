use crate::ClientKind;
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
	#[from]
	Custom(String),

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
	pub fn provider_connector(client_kind: ClientKind, cause: impl Into<String>) -> Self {
		Self::ProviderConnector(ProviderConnectorInfo {
			client_kind,
			cause: cause.into(),
		})
	}
}

// endregion: --- Impl

// region:    --- From AI Clients

#[allow(unused)]
#[derive(Debug)]
pub struct ProviderConnectorInfo {
	pub client_kind: ClientKind,
	pub cause: String,
}

impl From<async_openai::error::OpenAIError> for Error {
	fn from(raw_client_error: async_openai::error::OpenAIError) -> Self {
		Self::provider_connector(ClientKind::AsyncOpenAI, raw_client_error.to_string())
	}
}

impl From<ollama_rs::error::OllamaError> for Error {
	fn from(raw_client_error: ollama_rs::error::OllamaError) -> Self {
		Self::provider_connector(ClientKind::OllamaRs, raw_client_error.to_string())
	}
}

// endregion: --- From AI Clients

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
