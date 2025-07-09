use derive_more::{Display, From};
use reqwest::{StatusCode, header::HeaderMap};
use value_ext::JsonValueExtError;

pub type Result<T> = core::result::Result<T, Error>;

/// WebC submodule error.
#[allow(missing_docs)]
#[derive(Debug, From, Display)]
pub enum Error {
	#[display("Response content type '{content_type}' is not JSON as expected.")]
	ResponseFailedNotJson { content_type: String },

	#[display("Request failed with status code '{status}'. Response body:\n{body}")]
	ResponseFailedStatus {
		status: StatusCode,
		body: String,
		headers: HeaderMap,
	},

	// -- Utils
	#[display("JSON value extension error: {_0}")]
	#[from]
	JsonValueExt(JsonValueExtError),

	// -- Externals
	#[display("Reqwest error: {_0}")]
	#[from]
	Reqwest(reqwest::Error),

	#[display("Failed to clone EventSource request: {_0}")]
	#[from]
	EventSourceClone(reqwest_eventsource::CannotCloneRequestError),
}

// region:    --- Error Boilerplate

// NOTE: The manual Display implementation is removed as derive_more::Display handles it.
// impl core::fmt::Display for Error {
// 	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
// 		write!(fmt, "{self:?}")
// 	}
// }

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
