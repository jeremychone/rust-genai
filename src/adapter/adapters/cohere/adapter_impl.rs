use crate::adapter::cohere::CohereStream;
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatRole, ChatStream, ChatStreamResponse};
use crate::utils::x_value::XValue;
use crate::webc::{WebResponse, WebStream};
use crate::{ConfigSet, Error, Result};
use reqwest::RequestBuilder;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub struct CohereAdapter;

const BASE_URL: &str = "https://api.cohere.com/v1/";
const MODELS: &[&str] = &[
	"command-r-plus",
	"command-r",
	"command",
	"command-nightly",
	"command-light",
	"command-light-nightly",
];

impl Adapter for CohereAdapter {
	/// Note: For now returns the common ones (see above)
	async fn list_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(|| AdapterConfig::default().with_auth_env_name("COHERE_API_KEY"))
	}

	fn get_service_url(_kind: AdapterKind, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{BASE_URL}chat"),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let stream = matches!(service_type, ServiceType::ChatStream);

		let url = Self::get_service_url(kind, service_type);

		// -- api_key (this Adapter requires it)
		let api_key = get_api_key_resolver(kind, config_set)?;

		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let CohereChatRequestParts {
			preamble,
			message,
			chat_history,
		} = into_cohere_request_parts(kind, chat_req)?;

		// -- Build the basic payload
		let mut payload = json!({
			"model": model,
			"message": message,
			"stream": stream
		});

		if !chat_history.is_empty() {
			payload.x_insert("chat_history", chat_history)?;
		}
		if let Some(preamble) = preamble {
			payload.x_insert("preamble", preamble)?;
		}

		// -- Add supported ChatRequestOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("max_tokens", max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("p", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(_kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		let mut last_chat_history_item = body
			.x_take::<Vec<Value>>("chat_history")?
			.pop()
			.ok_or(Error::AdapterNoChatResponse)?;

		let content: Option<String> = last_chat_history_item.x_take("message")?;

		Ok(ChatResponse {
			content,
			..Default::default()
		})
	}

	fn to_chat_stream(_kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse> {
		let web_stream = WebStream::new_with_delimiter(reqwest_builder, "\n");
		let cohere_stream = CohereStream::new(web_stream);
		let chat_stream = ChatStream::from_inter_stream(cohere_stream);

		Ok(ChatStreamResponse { stream: chat_stream })
	}
}

// region:    --- Support

struct CohereChatRequestParts {
	/// The "system" in the cohere world
	preamble: Option<String>,
	/// The last user message
	message: String,
	/// The chat history (user and assistant, except last user message which is message)
	chat_history: Vec<Value>,
}

/// Takes the genai ChatMessages and build the System string and json Messages for Cohere.
/// - pop the last chat user message, and set it as message
/// - set the eventual `system` as first `preamble`
/// - add all of the system message in the 'preamble' (this might change when ChatReq will have `.system`)
/// - build the chat_history with the rest
fn into_cohere_request_parts(adapter_kind: AdapterKind, mut chat_req: ChatRequest) -> Result<CohereChatRequestParts> {
	let mut chat_history: Vec<Value> = Vec::new();
	let mut systems: Vec<String> = Vec::new();

	// -- Add the eventual system as pr
	if let Some(system) = chat_req.system {
		systems.push(system);
	}

	// -- Build extract the last user message
	let last_chat_msg = chat_req.messages.pop().ok_or(Error::AdapterChatReqHasNoMessages)?;
	if !matches!(last_chat_msg.role, ChatRole::User) {
		return Err(Error::AdapterLastChatMessageIsNoUser {
			actual_role: last_chat_msg.role,
		});
	}
	let message = last_chat_msg.content;

	// -- Build
	for msg in chat_req.messages {
		let content = msg.content;
		match msg.role {
			// for now, system and tool goes to system
			ChatRole::System => systems.push(content),
			ChatRole::User => chat_history.push(json! ({"role": "USER", "content": content})),
			ChatRole::Assistant => chat_history.push(json! ({"role": "CHATBOT", "content": content})),
			ChatRole::Tool => {
				return Err(Error::AdapterMessageRoleNotSupport {
					adapter_kind,
					role: ChatRole::Tool,
				})
			}
		}
	}

	// -- Build the preamble
	// Note: For now, we just concatenate the systems messages into the preamble as recommended by cohere
	//       Later, the ChatRequest should have `.system` property
	let preamble = if !systems.is_empty() {
		Some(systems.join("\n"))
	} else {
		None
	};

	Ok(CohereChatRequestParts {
		preamble,
		message,
		chat_history,
	})
}

// endregion: --- Support
