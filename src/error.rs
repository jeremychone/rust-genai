use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::{resolver, webc, ModelIden};
use derive_more::From;
use value_ext::JsonValueExtError;

/// GenAI main Result type alias (with genai::Error)
pub type Result<T> = core::result::Result<T, Error>;

/// Main GenAI error
#[derive(Debug, From)]
#[allow(missing_docs)]
pub enum Error {
	// -- Chat Input
	ChatReqHasNoMessages {
		model_iden: ModelIden,
	},
	LastChatMessageIsNotUser {
		model_iden: ModelIden,
		actual_role: ChatRole,
	},
	MessageRoleNotSupported {
		model_iden: ModelIden,
		role: ChatRole,
	},
	MessageContentTypeNotSupported {
		model_iden: ModelIden,
		cause: &'static str,
	},
	JsonModeWithoutInstruction,

	// -- Chat Output
	NoChatResponse {
		model_iden: ModelIden,
	},
	InvalidJsonResponseElement {
		info: &'static str,
	},

	// -- Embedding
	EmbeddingNotSupported {
		model_iden: ModelIden,
	},
	EmbeddingNotImplemented {
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

	// -- Externals
	#[from]
	EventSourceClone(reqwest_eventsource::CannotCloneRequestError),
	#[from]
	JsonValueExt(JsonValueExtError),
	ReqwestEventSource(reqwest_eventsource::Error),
	// Note: will probably need to remove this one to provide more context
	#[from]
	SerdeJson(serde_json::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
