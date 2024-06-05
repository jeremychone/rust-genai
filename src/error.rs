use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::webc;
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
	AdapterMessageRoleNotSupport {
		adapter_kind: AdapterKind,
		role: ChatRole,
	},

	// -- Provider
	ProviderHasNoDefaultApiKeyEnvName,

	// -- Stream
	StreamParse(serde_json::Error),
	ReqwestEventSource(reqwest_eventsource::Error),

	// -- Utils
	#[from]
	XValue(crate::utils::x_value::Error),
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

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
