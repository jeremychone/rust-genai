use crate::adapter::cohere::CohereStreamer;
use crate::adapter::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, MessageContent, MetaUsage,
};
use crate::webc::{WebResponse, WebStream};
use crate::{ClientConfig, ModelIden};
use crate::{Error, Result};
use reqwest::RequestBuilder;
use serde_json::{json, Value};
use value_ext::JsonValueExt;

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
	fn default_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		Some("COHERE_API_KEY")
	}

	/// Note: For now, it returns the common ones (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(_model_iden: ModelIden, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{BASE_URL}chat"),
		}
	}

	fn to_web_request_data(
		model_iden: ModelIden,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let model_name = model_iden.model_name.clone();

		let stream = matches!(service_type, ServiceType::ChatStream);

		let url = Self::get_service_url(model_iden.clone(), service_type);

		// -- api_key (this Adapter requires it)
		let api_key = get_api_key(model_iden.clone(), client_config)?;

		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let CohereChatRequestParts {
			preamble,
			message,
			chat_history,
		} = Self::into_cohere_request_parts(model_iden, chat_req)?;

		// -- Build the basic payload
		let mut payload = json!({
			"model": model_name.to_string(),
			"message": message,
			"stream": stream
		});

		if !chat_history.is_empty() {
			payload.x_insert("chat_history", chat_history)?;
		}
		if let Some(preamble) = preamble {
			payload.x_insert("preamble", preamble)?;
		}

		// -- Add supported ChatOptions
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

	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Get usage
		let usage = body.x_take("/meta/tokens").map(Self::into_usage).unwrap_or_default();

		// -- Get response
		let Some(mut last_chat_history_item) = body.x_take::<Vec<Value>>("chat_history")?.pop() else {
			return Err(Error::NoChatResponse { model_iden });
		};

		let content: Option<MessageContent> = last_chat_history_item
			.x_take::<Option<String>>("message")?
			.map(MessageContent::from);

		Ok(ChatResponse {
			content,
			model_iden,
			usage,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let web_stream = WebStream::new_with_delimiter(reqwest_builder, "\n");
		let cohere_stream = CohereStreamer::new(web_stream, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(cohere_stream);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}
}

// region:    --- Support

/// Support function
impl CohereAdapter {
	/// Convert usage from '/meta/tokens'
	/// ```json
	///  "tokens": {
	///    "input_tokens": 20,
	///    "output_tokens": 24
	///  }
	/// ```
	pub(super) fn into_usage(mut usage_value: Value) -> MetaUsage {
		let input_tokens: Option<i32> = usage_value.x_take("input_tokens").ok();
		let output_tokens: Option<i32> = usage_value.x_take("output_tokens").ok();

		// Compute total tokens
		let total_tokens = if input_tokens.is_some() || output_tokens.is_some() {
			Some(input_tokens.unwrap_or(0) + output_tokens.unwrap_or(0))
		} else {
			None
		};

		MetaUsage {
			input_tokens,
			output_tokens,
			total_tokens,
		}
	}

	/// Takes the GenAI ChatMessages and builds the system string and JSON messages for Cohere.
	/// - Pops the last chat user message and sets it as the message
	/// - Sets any eventual `system` as the first `preamble`
	/// - Adds all of the system messages into the 'preamble' (this might change when ChatReq has a `.system`)
	/// - Builds the chat history with the remaining messages
	fn into_cohere_request_parts(model_iden: ModelIden, mut chat_req: ChatRequest) -> Result<CohereChatRequestParts> {
		let mut chat_history: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		// -- Add the eventual system as preamble
		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Build and extract the last user message
		let last_chat_msg = chat_req.messages.pop().ok_or_else(|| Error::ChatReqHasNoMessages {
			model_iden: model_iden.clone(),
		})?;
		if !matches!(last_chat_msg.role, ChatRole::User) {
			return Err(Error::LastChatMessageIsNotUser {
				model_iden: model_iden.clone(),
				actual_role: last_chat_msg.role,
			});
		}

		// TODO: Needs to implement tool_calls
		let MessageContent::Text(message) = last_chat_msg.content else {
			return Err(Error::MessageContentTypeNotSupported {
				model_iden,
				cause: "Only MessageContent::Text supported for this model (for now)",
			});
		};

		// -- Build
		for msg in chat_req.messages {
			let MessageContent::Text(content) = msg.content else {
				return Err(Error::MessageContentTypeNotSupported {
					model_iden,
					cause: "Only MessageContent::Text supported for this model (for now)",
				});
			};

			match msg.role {
				// For now, system and tool go to the system
				ChatRole::System => systems.push(content),
				ChatRole::User => chat_history.push(json! ({"role": "USER", "content": content})),
				ChatRole::Assistant => chat_history.push(json! ({"role": "CHATBOT", "content": content})),
				ChatRole::Tool => {
					return Err(Error::MessageRoleNotSupported {
						model_iden,
						role: ChatRole::Tool,
					})
				}
			}
		}

		// -- Build the preamble
		// Note: For now, we just concatenate the system messages into the preamble as recommended by Cohere
		//       Later, the ChatRequest should have a `.system` property
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
}

struct CohereChatRequestParts {
	/// The "system" in the Cohere context
	preamble: Option<String>,
	/// The last user message
	message: String,
	/// The chat history (user and assistant, except the last user message which is the message)
	chat_history: Vec<Value>,
}

// endregion: --- Support
