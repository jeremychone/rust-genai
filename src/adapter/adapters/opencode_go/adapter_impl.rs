use crate::adapter::adapters::support::get_api_key;
use crate::adapter::anthropic::{AnthropicAdapter, AnthropicRequestParts};
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::json;
use value_ext::JsonValueExt;

pub struct OpenCodeGoAdapter;

impl OpenCodeGoAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OPENCODE_GO_API_KEY";
}

// region:    --- OpenCodeGoModelKind

/// Internal enum to dispatch wire format based on model name prefix.
/// MiniMax models use Anthropic protocol; all others use OpenAI protocol.
enum OpenCodeGoModelKind {
	OpenAI,
	Anthropic,
}

impl OpenCodeGoModelKind {
	fn from_model_name(name: &str) -> Self {
		if name.to_lowercase().starts_with("minimax-") {
			Self::Anthropic
		} else {
			Self::OpenAI
		}
	}
}

// endregion: --- OpenCodeGoModelKind

impl Adapter for OpenCodeGoAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_auth() -> AuthData {
		AuthData::from_env("OPENCODE_GO_API_KEY")
	}

	fn default_endpoint() -> Endpoint {
		Endpoint::from_static("https://opencode.ai/zen/go/v1/")
	}

	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		let mut models = OpenAIAdapter::list_model_names_for_end_target(kind, endpoint, auth).await?;
		// Hardcoded fallback: ensure MiniMax models are present even if the API
		// listing omits them. Task 1 validation confirmed they appear, but we
		// keep this as a safety net for future API changes.
		for minimax_model in ["minimax-m2.5", "minimax-m2.7"] {
			if !models.iter().any(|m| m == minimax_model) {
				models.push(minimax_model.to_string());
			}
		}
		Ok(models)
	}

	fn get_service_url(model: &ModelIden, _service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let (_, model_name) = model.model_name.namespace_and_name();
		let model_kind = OpenCodeGoModelKind::from_model_name(model_name);
		let suffix = match model_kind {
			OpenCodeGoModelKind::OpenAI => "chat/completions",
			OpenCodeGoModelKind::Anthropic => "messages",
		};
		Ok(format!("{base_url}{suffix}"))
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;
		let (_, model_name) = model.model_name.namespace_and_name();
		let model_kind = OpenCodeGoModelKind::from_model_name(model_name);

		match model_kind {
			OpenCodeGoModelKind::OpenAI => {
				OpenAIAdapter::util_to_web_request_data(
					ServiceTarget { endpoint, auth, model },
					service_type,
					chat_req,
					options_set,
					None,
				)
			}
			OpenCodeGoModelKind::Anthropic => {
				let model_name = model_name.to_string();

				let AnthropicRequestParts { system, messages, tools } =
					AnthropicAdapter::into_anthropic_request_parts(chat_req)?;

				let stream = matches!(service_type, ServiceType::ChatStream);
				let mut payload = json!({
					"model": model_name,
					"messages": messages,
					"stream": stream,
				});

				if let Some(system) = system {
					payload.x_insert("system", system)?;
				}

				if let Some(tools) = tools {
					payload.x_insert("tools", tools)?;
				}

				if let Some(temperature) = options_set.temperature() {
					payload.x_insert("temperature", temperature)?;
				}

				if let Some(top_p) = options_set.top_p() {
					payload.x_insert("top_p", top_p)?;
				}

				if !options_set.stop_sequences().is_empty() {
					payload.x_insert("stop_sequences", options_set.stop_sequences())?;
				}

				if let Some(max_tokens) = options_set.max_tokens() {
					payload.x_insert("max_tokens", max_tokens)?;
				}

				// MiniMax /v1/messages requires `x-api-key` (Bearer returns 401).
				let api_key = get_api_key(auth, &model)?;
				let headers = Headers::from(("x-api-key".to_string(), api_key));

				let url = Self::get_service_url(&model, service_type, endpoint)?;

				Ok(WebRequestData { url, headers, payload })
			}
		}
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let (_, model_name) = model_iden.model_name.namespace_and_name();
		let model_kind = OpenCodeGoModelKind::from_model_name(model_name);

		match model_kind {
			OpenCodeGoModelKind::OpenAI => OpenAIAdapter::to_chat_response(model_iden, web_response, options_set),
			OpenCodeGoModelKind::Anthropic => AnthropicAdapter::to_chat_response(model_iden, web_response, options_set),
		}
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let (_, model_name) = model_iden.model_name.namespace_and_name();
		let model_kind = OpenCodeGoModelKind::from_model_name(model_name);

		match model_kind {
			OpenCodeGoModelKind::OpenAI => OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			OpenCodeGoModelKind::Anthropic => AnthropicAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
		}
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: EmbedRequest,
		_options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::OpenCodeGo,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::OpenCodeGo,
			feature: "embeddings".to_string(),
		})
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ServiceTarget;
	use crate::adapter::{Adapter, ServiceType};
	use crate::chat::{ChatOptionsSet, ChatRequest};
	use crate::embed::{EmbedOptionsSet, EmbedRequest};
	use crate::resolver::AuthData;

	fn test_target(model_name: &str) -> ServiceTarget {
		ServiceTarget {
			endpoint: OpenCodeGoAdapter::default_endpoint(),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::OpenCodeGo, model_name),
		}
	}

	fn make_request(model_name: &str, service_type: ServiceType) -> WebRequestData {
		OpenCodeGoAdapter::to_web_request_data(
			test_target(model_name),
			service_type,
			ChatRequest::from_user("hello"),
			ChatOptionsSet::default(),
		)
		.expect("to_web_request_data should succeed")
	}

	#[test]
	fn test_url_openai_path() {
		let data = make_request("glm-5", ServiceType::Chat);
		assert!(
			data.url.ends_with("chat/completions"),
			"OpenAI path URL should end with chat/completions: {}",
			data.url
		);
	}

	#[test]
	fn test_url_minimax_path() {
		let data = make_request("minimax-m2.5", ServiceType::Chat);
		assert!(
			data.url.ends_with("messages"),
			"Minimax path URL should end with messages: {}",
			data.url
		);
	}

	#[test]
	fn test_auth_header_openai() {
		let data = make_request("glm-5", ServiceType::Chat);
		let auth = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("Authorization"))
			.map(|(_, v)| v.as_str());
		assert_eq!(auth, Some("Bearer test-key"));
	}

	#[test]
	fn test_auth_header_minimax() {
		let data = make_request("minimax-m2.5", ServiceType::Chat);
		let auth = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("Authorization"))
			.map(|(_, v)| v.as_str());
		assert_eq!(auth, None, "Minimax should not have Authorization header");
		let x_key = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("x-api-key"))
			.map(|(_, v)| v.as_str());
		assert_eq!(x_key, Some("test-key"));
	}

	#[test]
	fn test_no_x_api_key_in_openai_path() {
		let data = make_request("glm-5", ServiceType::Chat);
		let x_key = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("x-api-key"))
			.map(|(_, v)| v.as_str());
		assert_eq!(x_key, None, "OpenAI path should not have x-api-key header");
	}

	#[test]
	fn test_x_api_key_in_minimax_path() {
		let data = make_request("minimax-m2.5", ServiceType::Chat);
		let x_key = data
			.headers
			.iter()
			.find(|(k, _)| k.eq_ignore_ascii_case("x-api-key"))
			.map(|(_, v)| v.as_str());
		assert_eq!(x_key, Some("test-key"));
	}

	#[test]
	fn test_payload_model_name_openai() {
		let data = make_request("glm-5", ServiceType::Chat);
		let model = data.payload.get("model").and_then(|v| v.as_str());
		assert_eq!(model, Some("glm-5"));
	}

	#[test]
	fn test_payload_model_name_minimax() {
		let data = make_request("minimax-m2.5", ServiceType::Chat);
		let model = data.payload.get("model").and_then(|v| v.as_str());
		assert_eq!(model, Some("minimax-m2.5"));
	}

	#[test]
	fn test_payload_messages_array() {
		for (name, model_name) in [("OpenAI", "glm-5"), ("Minimax", "minimax-m2.5")] {
			let data = make_request(model_name, ServiceType::Chat);
			let messages = data.payload.get("messages").and_then(|v| v.as_array());
			assert!(
				messages.is_some(),
				"{name} payload should have messages array"
			);
			let messages = messages.unwrap();
			assert!(!messages.is_empty(), "{name} messages array should not be empty");
			let last = messages.last().unwrap();
			let content = last.get("content").and_then(|v| v.as_str());
			assert_eq!(content, Some("hello"), "{name} last message content mismatch");
		}
	}

	#[test]
	fn test_payload_stream_false_for_chat() {
		for (name, model_name) in [("OpenAI", "glm-5"), ("Minimax", "minimax-m2.5")] {
			let data = make_request(model_name, ServiceType::Chat);
			let stream = data.payload.get("stream").and_then(|v| v.as_bool());
			assert_eq!(stream, Some(false), "{name} stream should be false for Chat");
		}
	}

	#[test]
	fn test_payload_stream_true_for_chat_stream() {
		for (name, model_name) in [("OpenAI", "glm-5"), ("Minimax", "minimax-m2.5")] {
			let data = make_request(model_name, ServiceType::ChatStream);
			let stream = data.payload.get("stream").and_then(|v| v.as_bool());
			assert_eq!(stream, Some(true), "{name} stream should be true for ChatStream");
		}
	}

	#[test]
	fn test_minimax_prefix_case_insensitive() {
		let data = make_request("MINIMAX-M2.5", ServiceType::Chat);
		assert!(
			data.url.ends_with("messages"),
			"MINIMAX-M2.5 should route to messages URL: {}",
			data.url
		);

		let data = make_request("Minimax-m2.5", ServiceType::Chat);
		assert!(
			data.url.ends_with("messages"),
			"Minimax-m2.5 should route to messages URL: {}",
			data.url
		);
	}

	#[test]
	fn test_embed_not_supported() {
		let result = OpenCodeGoAdapter::to_embed_request_data(
			test_target("glm-5"),
			EmbedRequest::new("test"),
			EmbedOptionsSet::default(),
		);
		assert!(result.is_err(), "embed should not be supported");
		match result.unwrap_err() {
			Error::AdapterNotSupported { adapter_kind, feature } => {
				assert_eq!(adapter_kind, AdapterKind::OpenCodeGo);
				assert_eq!(feature, "embeddings");
			}
			_ => panic!("Expected AdapterNotSupported error"),
		}
	}
}

// endregion: --- Tests
