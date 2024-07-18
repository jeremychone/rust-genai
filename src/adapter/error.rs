use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::{resolver, webc};
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
	// -- Adapter Chat Request
	ChatReqHasNoMessages,
	LastChatMessageIsNoUser {
		actual_role: ChatRole,
	},

	// -- Adapter Chat Response
	NoChatResponse,

	// -- Adapter
	RequiresApiKey {
		adapter_kind: AdapterKind,
	},
	MessageRoleNotSupport {
		adapter_kind: AdapterKind,
		role: ChatRole,
	},
	HasNoDefaultApiKeyEnvName,
	NoAuthResolver {
		adapter_kind: AdapterKind,
	},
	AuthResolverNoAuthData {
		adapter_kind: AdapterKind,
	},

	// -- Stream
	StreamParse(serde_json::Error),
	StreamEventError(serde_json::Value),
	WebStream,

	// -- Modules
	#[from]
	Webc(webc::Error),
	#[from]
	Resolver(resolver::Error),

	// -- Utils
	#[from]
	XValue(crate::utils::x_value::Error),

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
