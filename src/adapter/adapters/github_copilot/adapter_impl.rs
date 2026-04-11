use super::copilot_types::{
	COPILOT_DEFAULT_ENDPOINT, COPILOT_EDITOR_PLUGIN_VERSION, COPILOT_EDITOR_VERSION, COPILOT_INTEGRATION_ID,
	COPILOT_USER_AGENT,
};
use crate::ModelIden;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Headers, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct GithubCopilotAdapter;

impl Adapter for GithubCopilotAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = None;

	fn default_auth() -> AuthData {
		AuthData::None
	}

	fn default_endpoint() -> Endpoint {
		Endpoint::from_static(COPILOT_DEFAULT_ENDPOINT)
	}

	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData) -> Result<Vec<String>> {
		tracing::warn!("GitHub Copilot does not expose a model catalog endpoint");
		Ok(Vec::new())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OpenAIAdapter::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let target = with_stripped_publisher_prefix_model(target);
		let mut data = OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)?;
		data.headers.merge(copilot_identity_headers());
		Ok(data)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	fn to_embed_request_data(
		service_target: ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}

// region:    --- Support

fn with_stripped_publisher_prefix_model(target: ServiceTarget) -> ServiceTarget {
	let ServiceTarget { endpoint, auth, model } = target;
	let stripped_model_name = strip_publisher_prefix(model.model_name.namespace_and_name().1);

	ServiceTarget {
		endpoint,
		auth,
		model: model.from_name(stripped_model_name),
	}
}

fn strip_publisher_prefix(model: &str) -> &str {
	model.split_once('/').map(|(_, model_name)| model_name).unwrap_or(model)
}

fn copilot_identity_headers() -> Headers {
	Headers::from([
		("User-Agent", COPILOT_USER_AGENT),
		("Editor-Version", COPILOT_EDITOR_VERSION),
		("Editor-Plugin-Version", COPILOT_EDITOR_PLUGIN_VERSION),
		("Copilot-Integration-Id", COPILOT_INTEGRATION_ID),
	])
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::{Adapter, ServiceType};
	use crate::chat::{ChatOptionsSet, ChatRequest};
	use crate::resolver::AuthData;

	fn test_target(model_name: &str) -> ServiceTarget {
		ServiceTarget {
			endpoint: GithubCopilotAdapter::default_endpoint(),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::GithubCopilot, model_name),
		}
	}

	fn make_request(model_name: &str, service_type: ServiceType) -> WebRequestData {
		GithubCopilotAdapter::to_web_request_data(
			test_target(model_name),
			service_type,
			ChatRequest::from_user("hello"),
			ChatOptionsSet::default(),
		)
		.expect("to_web_request_data should succeed")
	}

	fn header_value<'a>(data: &'a WebRequestData, header_name: &str) -> Option<&'a str> {
		data.headers
			.iter()
			.find(|(name, _)| name.eq_ignore_ascii_case(header_name))
			.map(|(_, value)| value.as_str())
	}

	#[test]
	fn test_default_endpoint_is_copilot_api() {
		assert!(
			GithubCopilotAdapter::default_endpoint()
				.base_url()
				.starts_with(COPILOT_DEFAULT_ENDPOINT)
		);
	}

	#[test]
	fn test_copilot_identity_headers_present() {
		let data = make_request("openai/gpt-4o", ServiceType::Chat);

		assert_eq!(header_value(&data, "User-Agent"), Some(COPILOT_USER_AGENT));
		assert_eq!(header_value(&data, "Editor-Version"), Some(COPILOT_EDITOR_VERSION));
		assert_eq!(
			header_value(&data, "Editor-Plugin-Version"),
			Some(COPILOT_EDITOR_PLUGIN_VERSION)
		);
		assert_eq!(
			header_value(&data, "Copilot-Integration-Id"),
			Some(COPILOT_INTEGRATION_ID)
		);
	}

	#[test]
	fn test_old_github_models_header_absent() {
		let data = make_request("openai/gpt-4o", ServiceType::Chat);

		assert!(
			data.headers.iter().all(|(_, value)| value != "application/vnd.github+json"),
			"legacy GitHub Models accept header should be absent"
		);
	}

	#[test]
	fn test_model_name_publisher_prefix_stripped() {
		let data = make_request("openai/gpt-4o", ServiceType::Chat);

		assert_eq!(
			data.payload.get("model").and_then(|value| value.as_str()),
			Some("gpt-4o")
		);
	}

	#[test]
	fn test_model_name_no_prefix_unchanged() {
		let data = make_request("gpt-4o", ServiceType::Chat);

		assert_eq!(
			data.payload.get("model").and_then(|value| value.as_str()),
			Some("gpt-4o")
		);
	}

	#[test]
	fn test_url_construction() {
		let data = make_request("openai/gpt-4o", ServiceType::Chat);

		assert!(data.url.starts_with(COPILOT_DEFAULT_ENDPOINT));
		assert_eq!(data.url, "https://api.githubcopilot.com/chat/completions");
	}
}

// endregion: --- Tests
