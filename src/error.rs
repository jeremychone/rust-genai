use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::ModelInfo;
use crate::{resolver, webc};
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
	// -- Chat Input
	ChatReqHasNoMessages {
		model_info: ModelInfo,
	},
	LastChatMessageIsNoUser {
		model_info: ModelInfo,
		actual_role: ChatRole,
	},
	MessageRoleNotSupported {
		model_info: ModelInfo,
		role: ChatRole,
	},

	// -- Chat Output
	NoChatResponse {
		model_info: ModelInfo,
	},

	// -- Auth
	RequiresApiKey {
		model_info: ModelInfo,
	},
	NoAuthResolver {
		model_info: ModelInfo,
	},
	AuthResolverNoAuthData {
		model_info: ModelInfo,
	},

	// -- Web Call error
	WebAdapterCall {
		adapter_kind: AdapterKind,
		webc_error: webc::Error,
	},
	WebModelCall {
		model_info: ModelInfo,
		webc_error: webc::Error,
	},

	// -- Chat Stream
	StreamParse {
		model_info: ModelInfo,
		serde_error: serde_json::Error,
	},
	StreamEventError {
		model_info: ModelInfo,
		body: serde_json::Value,
	},
	WebStream {
		model_info: ModelInfo,
		cause: String,
	},

	// -- Modules
	#[from]
	Resolver {
		model_info: ModelInfo,
		resolver_error: resolver::Error,
	},

	// -- Utils
	#[from]
	XValue(crate::support::value_ext::Error),

	// -- Externals
	#[from]
	EventSourceClone(reqwest_eventsource::CannotCloneRequestError),
	ReqwestEventSource(reqwest_eventsource::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
