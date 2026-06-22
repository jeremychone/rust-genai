use crate::adapter::adapters::anthropic::{AnthropicAdapter, AnthropicStreamer};
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{EventSourceStream, WebClient, WebResponse};
use crate::{ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct MinimaxAdapter;

impl MinimaxAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "MINIMAX_API_KEY";
}

impl Adapter for MinimaxAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		const BASE_URL: &str = "https://api.minimax.io/anthropic/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth(_kind: AdapterKind) -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	async fn all_model_names(kind: AdapterKind, endpoint: Endpoint, auth: AuthData, web_client: &WebClient) -> Result<Vec<String>> {
		// NOTE: Minimax uses the same Anthropic protocol endpoint for listing models.
		// For now, we return an empty list. This can be extended later.
		let _ = (kind, endpoint, auth, web_client);
		Ok(vec![])
	}

	fn get_service_url(_model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let url = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}messages"),
			ServiceType::Embed => format!("{base_url}embeddings"),
		};
		Ok(url)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;
		AnthropicAdapter::build_web_request_data(endpoint, auth, model, service_type, chat_req, options_set)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		AnthropicAdapter::build_chat_response(model_iden, web_response)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSourceStream::new(reqwest_builder);
		let anthropic_stream = AnthropicStreamer::new(event_source, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(anthropic_stream);
		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: EmbedRequest,
		_options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: AdapterKind::MiniMax,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: AdapterKind::MiniMax,
			feature: "embeddings".to_string(),
		})
	}
}
