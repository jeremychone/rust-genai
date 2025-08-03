use crate::ModelIden;
use crate::adapter::openai::{OpenAIAdapter, ToWebRequestCustom};
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

/// The Fireworks API is mostly compatible with the OpenAI API.
///
/// NOTE: This adapter will add `accounts/fireworks/models/`
///       if the model name does not contain a `/`.
///
/// For example, `qwen3-30b-a3b` becomes `accounts/fireworks/models/qwen3-30b-a3b`.
///
/// Since this adapter is activated only when `fireworks` is in the model name,
/// or if the model is namespaced with `fireworks::`, you can simply use
/// `fireworks::qwen3-30b-a3b` to resolve to `accounts/fireworks/models/qwen3-30b-a3b`.
///
/// However, if the model name has a `/`, then it is assumed to be one recognized by the fireworks.ai service.
pub struct FireworksAdapter;

/// For fireworks, perhaps to many to list.
/// Might do the top one later.
/// But model to adapter kind happen if "fireworks is part of the model name
pub(in crate::adapter) const MODELS: &[&str] = &[];

impl FireworksAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "FIREWORKS_API_KEY";
}

impl Adapter for FireworksAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.fireworks.ai/inference/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		OpenAIAdapter::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		mut target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		// NOTE: Here we do the simplification logic about the model
		//       e.g., adding the prefix `accounts/fireworks/models/` if the model name does not contain any `/`
		if !target.model.model_name.contains('/') {
			target.model = target.model.from_name(format!(
				"accounts/fireworks/models/{}",
				target.model.model_name.as_model_name_and_namespace().0
			))
		}
		// NOTE: Fireworks max_tokens is set at 2K by default, which is unpractical for most task.
		//       However, Fireworks allows max tokens above the model limit; it will simply be capped at the model's maximum by Fireworks, so it won't fail.
		//       See: https://fireworks.ai/docs/faq-new/models-inference/what-are-the-maximum-completion-token-limits-for-models-and-can-they-be-increase
		// NOTE: The `genai` strategy is to set a large max_tokens value, letting the model enforce its own lower limit by default to avoid unpleasant and confusing surprises.
		//       Users can use [`ChatOptions`] to specify a specific max_tokens value.
		// NOTE: As mentioned in the Fireworks FAQ above, typically, for Fireworks-hosted models the top max_tokens is equal to the context window.
		//       Since, Qwen3 models are at 256k, so we will use this upper bound (without going to the 1M/10M of Llama 4).
		let custom = ToWebRequestCustom {
			default_max_tokens: Some(256_000),
		};

		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, Some(custom))
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
