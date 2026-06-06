use crate::ModelIden;
use crate::adapter::adapters::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

/// The OpenRouter API is compatible with the OpenAI API.
/// NOTE: This adapter is activated for namespaced model names (e.g., `open_router::openai/gpt-4.1`)
pub struct CustomAdapter;

// TODO FOR LLM
// Implement CustomAdapter::get_endpoint(n: u16) -> Option<Endpoint>
// Implement CustomAdapter::get_auth(n: u16) -> Option<AuthData>

impl CustomAdapter {
	/// Returns the endpoint for a custom adapter number `n` from the environment variable `GENAI_{n}_ENDPOINT`.
	pub fn get_endpoint(n: u8) -> Option<Endpoint> {
		let env_name = format!("GENAI_{}_ENDPOINT", n);

		// making sure it ends with `/` as expected by the adapters
		let mut endpoint = std::env::var(&env_name).ok()?;
		if !endpoint.ends_with("/") {
			endpoint.push('/');
		}

		Some(Endpoint::from_owned(endpoint))
	}

	/// Returns the auth for a custom adapter number `n` from the environment variable `GENAI_{n}_API_KEY`.
	/// Returns `None` if the environment variable is not set.
	pub fn get_auth(n: u8) -> Option<AuthData> {
		let env_name = format!("GENAI_{}_API_KEY", n);
		std::env::var(&env_name).ok().map(|_| AuthData::from_env(env_name))
	}
}

impl Adapter for CustomAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = None;

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		// returns empty, cannot determine at this stage
		if let AdapterKind::Custom(n) = _kind {
			Self::get_endpoint(n).unwrap_or_else(|| Endpoint::from_static(""))
		} else {
			Endpoint::from_static("")
		}
	}

	fn default_auth(_kind: AdapterKind) -> AuthData {
		// returns empty, cannot determine at this stage
		if let AdapterKind::Custom(n) = _kind {
			Self::get_auth(n).unwrap_or(AuthData::None)
		} else {
			AuthData::None
		}
	}

	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		OpenAIAdapter::list_model_names_for_end_target(kind, endpoint, auth).await
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
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)
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
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}
