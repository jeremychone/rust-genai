use derive_more::{Display, From};
use reqwest::{StatusCode, header::HeaderMap};
use value_ext::JsonValueExtError;

pub type Result<T> = core::result::Result<T, Error>;

/// WebC submodule error.
#[allow(missing_docs)]
#[derive(Debug, From, Display)]
pub enum Error {
	#[display("Response content type '{content_type}' is not JSON as expected. Response body:\n{body}")]
	ResponseFailedNotJson { content_type: String, body: String },

	#[display("Response was invalid json. Cause:\n'{cause}'\nBody:\n{body}")]
	ResponseFailedInvalidJson { body: String, cause: String },

	#[display("Request failed with status code '{status}'. Response body:\n{body}")]
	ResponseFailedStatus {
		status: StatusCode,
		body: String,
		headers: Box<HeaderMap>,
	},

	// -- Utils
	#[display("JSON value extension error: {_0}")]
	#[from]
	JsonValueExt(JsonValueExtError),

	// -- Externals
	#[display("Reqwest error: {_0}")]
	#[from]
	Reqwest(reqwest::Error),
}

/// Accessors
impl Error {
	/// The HTTP status behind this error, when there is one.
	///
	/// `ResponseFailedStatus` carries it directly; a `Reqwest` error may
	/// also carry one (for instance from `error_for_status`). Everything
	/// else — a non-JSON body, an unparsable body, a connection failure —
	/// has no status to report.
	pub fn status(&self) -> Option<StatusCode> {
		match self {
			Error::ResponseFailedStatus { status, .. } => Some(*status),
			Error::Reqwest(reqwest_error) => reqwest_error.status(),
			_ => None,
		}
	}

	/// Response headers returned with a failed HTTP status, when available.
	///
	/// These headers commonly carry provider retry guidance such as
	/// `retry-after`, `retry-after-ms`, and `x-should-retry`. Errors produced
	/// before a response is received, and `reqwest::Error`, do not retain a
	/// response header map here.
	pub fn headers(&self) -> Option<&HeaderMap> {
		match self {
			Error::ResponseFailedStatus { headers, .. } => Some(headers),
			_ => None,
		}
	}
}

// region:    --- Error Boilerplate

// NOTE: The manual Display implementation is removed as derive_more::Display handles it.
// impl core::fmt::Display for Error {
// 	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
// 		write!(fmt, "{self:?}")
// 	}
// }

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
	use super::*;
	use reqwest::header::{HeaderValue, RETRY_AFTER};

	#[test]
	fn response_status_headers_are_exposed() {
		let mut headers = HeaderMap::new();
		headers.insert(RETRY_AFTER, HeaderValue::from_static("5"));
		let error = Error::ResponseFailedStatus {
			status: StatusCode::TOO_MANY_REQUESTS,
			body: "slow down".to_string(),
			headers: Box::new(headers),
		};

		assert_eq!(
			error.headers().and_then(|headers| headers.get(RETRY_AFTER)),
			Some(&HeaderValue::from_static("5"))
		);
	}
}

// endregion: --- Error Boilerplate
