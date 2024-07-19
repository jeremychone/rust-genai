use crate::adapter::AdapterKind;
use crate::chat::ChatRole;
use crate::{resolver, webc};
use derive_more::From;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, From)]
pub enum Error {
	// -- Chat Input
	ChatReqHasNoMessages {
		adapter_kind: AdapterKind,
	},
	LastChatMessageIsNoUser {
		actual_role: ChatRole,
	},
	MessageRoleNotSupported {
		adapter_kind: AdapterKind,
		role: ChatRole,
	},

	// -- Chat Output
	NoChatResponse {
		adapter_kind: AdapterKind,
	},

	// -- Auth
	RequiresApiKey {
		adapter_kind: AdapterKind,
	},
	NoAuthResolver {
		adapter_kind: AdapterKind,
	},
	AuthResolverNoAuthData {
		adapter_kind: AdapterKind,
	},

	// -- Web Call error
	WebCall {
		adapter_kind: AdapterKind,
		webc_error: webc::Error,
	},

	// -- Chat Stream
	StreamParse(serde_json::Error),
	StreamEventError(serde_json::Value),
	WebStream {
		adapter_kind: AdapterKind,
		cause: String,
	},

	// -- Modules
	// #[from]
	// Webc(webc::Error),
	#[from]
	Resolver(resolver::Error),

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
