use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::{resolver, webc, ModelIden};
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
	// -- Chat Input
	ChatReqHasNoMessages {
		model_iden: ModelIden,
	},
	LastChatMessageIsNoUser {
		model_iden: ModelIden,
		actual_role: ChatRole,
	},
	MessageRoleNotSupported {
		model_iden: ModelIden,
		role: ChatRole,
	},
	JsonModeWithoutInstruction,

	// -- Chat Output
	NoChatResponse {
		model_iden: ModelIden,
	},

	// -- Auth
	RequiresApiKey {
		model_iden: ModelIden,
	},
	NoAuthResolver {
		model_iden: ModelIden,
	},
	NoAuthData {
		model_iden: ModelIden,
	},

	// -- ModelMapper
	ModelMapperFailed {
		model_iden: ModelIden,
		cause: resolver::Error,
	},

	// -- Web Call error
	WebAdapterCall {
		adapter_kind: AdapterKind,
		webc_error: webc::Error,
	},
	WebModelCall {
		model_iden: ModelIden,
		webc_error: webc::Error,
	},

	// -- Chat Stream
	StreamParse {
		model_iden: ModelIden,
		serde_error: serde_json::Error,
	},
	StreamEventError {
		model_iden: ModelIden,
		body: serde_json::Value,
	},
	WebStream {
		model_iden: ModelIden,
		cause: String,
	},

	// -- Modules
	Resolver {
		model_iden: ModelIden,
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
