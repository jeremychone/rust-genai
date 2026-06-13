//! Derivation of the `error.type` attribute from a genai [`Error`].
//!
//! Per the GenAI spec, `error.type` SHOULD be the error code returned by the
//! provider (e.g. an HTTP status), the canonical exception name, or another
//! low-cardinality identifier. HTTP failures are reported as their numeric
//! status code; everything else maps to a short, stable, snake-case identifier.

use crate::Error;
use crate::webc;

/// Returns the `error.type` value for the given error.
pub fn error_type(error: &Error) -> String {
	match error {
		Error::WebModelCall { webc_error, .. } | Error::WebAdapterCall { webc_error, .. } => {
			webc_error_type(webc_error)
		}
		Error::HttpError { status, .. } => status.as_u16().to_string(),

		// -- Input / request shaping
		Error::ChatReqHasNoMessages { .. } => "chat_req_has_no_messages".to_string(),
		Error::LastChatMessageIsNotUser { .. } => "last_chat_message_is_not_user".to_string(),
		Error::MessageRoleNotSupported { .. } => "message_role_not_supported".to_string(),
		Error::MessageContentTypeNotSupported { .. } => "message_content_type_not_supported".to_string(),
		Error::JsonModeWithoutInstruction => "json_mode_without_instruction".to_string(),
		Error::VerbosityParsing { .. } => "verbosity_parsing".to_string(),
		Error::ReasoningParsingError { .. } => "reasoning_parsing".to_string(),
		Error::ServiceTierParsing { .. } => "service_tier_parsing".to_string(),
		Error::PromptCacheRetentionParsing { .. } => "prompt_cache_retention_parsing".to_string(),

		// -- Output / response shaping
		Error::NoChatResponse { .. } => "no_chat_response".to_string(),
		Error::InvalidJsonResponseElement { .. } => "invalid_json_response_element".to_string(),
		Error::ChatResponseGeneration { .. } => "chat_response_generation".to_string(),
		Error::ChatResponse { .. } => "chat_response".to_string(),

		// -- Auth
		Error::RequiresApiKey { .. } => "requires_api_key".to_string(),
		Error::NoAuthResolver { .. } => "no_auth_resolver".to_string(),
		Error::NoAuthData { .. } => "no_auth_data".to_string(),

		// -- Mapping / resolving
		Error::ModelMapperFailed { .. } => "model_mapper_failed".to_string(),
		Error::Resolver { .. } => "resolver".to_string(),

		// -- Stream
		Error::StreamParse { .. } => "stream_parse".to_string(),
		Error::WebStream { .. } => "web_stream".to_string(),

		// -- Adapter support
		Error::AdapterNotSupported { .. } => "adapter_not_supported".to_string(),
		Error::AdapterKindMismatch { .. } => "adapter_kind_mismatch".to_string(),

		// -- Internals / externals
		Error::Internal(_) => "internal".to_string(),
		Error::JsonValueExt(_) => "json_value_ext".to_string(),
		Error::SerdeJson(_) => "serde_json".to_string(),
	}
}

/// Maps a [`webc::Error`] to an `error.type` value, preferring the HTTP status
/// code when one is available.
fn webc_error_type(error: &webc::Error) -> String {
	match error {
		webc::Error::ResponseFailedStatus { status, .. } => status.as_u16().to_string(),
		webc::Error::Reqwest(reqwest_error) => {
			if let Some(status) = reqwest_error.status() {
				status.as_u16().to_string()
			} else if reqwest_error.is_timeout() {
				"timeout".to_string()
			} else if reqwest_error.is_connect() {
				"connect".to_string()
			} else {
				"reqwest".to_string()
			}
		}
		webc::Error::ResponseFailedNotJson { .. } => "response_not_json".to_string(),
		webc::Error::ResponseFailedInvalidJson { .. } => "response_invalid_json".to_string(),
		webc::Error::JsonValueExt(_) => "json_value_ext".to_string(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ModelIden;
	use crate::adapter::AdapterKind;
	use reqwest::StatusCode;
	use reqwest::header::HeaderMap;

	fn model() -> ModelIden {
		ModelIden::from_static(AdapterKind::OpenAI, "gpt-4o")
	}

	#[test]
	fn test_otel_error_type_http_status() {
		let error = Error::HttpError {
			status: StatusCode::TOO_MANY_REQUESTS,
			canonical_reason: "Too Many Requests".to_string(),
			body: String::new(),
		};
		assert_eq!(error_type(&error), "429");
	}

	#[test]
	fn test_otel_error_type_web_model_call_status() {
		let webc_error = webc::Error::ResponseFailedStatus {
			status: StatusCode::INTERNAL_SERVER_ERROR,
			body: String::new(),
			headers: Box::new(HeaderMap::new()),
		};
		let error = Error::WebModelCall {
			model_iden: model(),
			webc_error,
		};
		assert_eq!(error_type(&error), "500");
	}

	#[test]
	fn test_otel_error_type_low_cardinality_names() {
		assert_eq!(error_type(&Error::Internal("boom".to_string())), "internal");
		assert_eq!(
			error_type(&Error::NoChatResponse { model_iden: model() }),
			"no_chat_response"
		);
		assert_eq!(
			error_type(&Error::JsonModeWithoutInstruction),
			"json_mode_without_instruction"
		);
	}
}
