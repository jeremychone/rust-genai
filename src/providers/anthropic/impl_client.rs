use crate::anthropic::streamer::{AnthropicMessagesStream, AnthropicStreamEvent};
use crate::anthropic::AnthropicProvider;
use crate::providers::support::{get_api_key_from_config, Provider};
use crate::utils::x_value::XValue;
use crate::webc::Response;
use crate::{ChatMessage, ChatRequest, ChatResponse, ChatRole, ChatStream, Client, Result, StreamItem};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};

const MAX_TOKENS: u32 = 1024;
const ANTRHOPIC_VERSION: &str = "2023-06-01";

#[async_trait]
impl Client for AnthropicProvider {
	async fn list_models(&self) -> Result<Vec<String>> {
		// TODO:
		Ok(vec![])
	}

	// see: https://docs.anthropic.com/en/api/messages
	async fn exec_chat(&self, model: &str, req: ChatRequest) -> Result<ChatResponse> {
		let WebRequestData { headers, payload } = self.to_anthrophic_web_request_data(model, req, false)?;
		let headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

		let response = self.web_client().do_post("messages", &headers, payload).await?;

		let chat_res = into_genai_chat_response(response)?;

		Ok(chat_res)
	}

	async fn exec_chat_stream(&self, model: &str, req: ChatRequest) -> Result<crate::ChatStream> {
		let WebRequestData { headers, payload } = self.to_anthrophic_web_request_data(model, req, true)?;

		let headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

		let event_source = self.web_client().do_post_stream("messages", &headers, payload).await?;

		let anthropic_stream = AnthropicMessagesStream::new(event_source);

		let stream = anthropic_stream.filter_map(|an_stream_event| async move {
			match an_stream_event {
				Err(err) => Some(Err(err)),
				Ok(AnthropicStreamEvent::BlockDelta(content)) => Some(Ok(StreamItem { content: Some(content) })),
				_ => None,
			}
		});

		Ok(ChatStream {
			stream: Box::pin(stream),
		})
	}
}

// region:    --- Support Implementations

struct WebRequestData {
	headers: Vec<(String, String)>,
	payload: Value,
}

impl AnthropicProvider {
	fn to_anthrophic_web_request_data(&self, model: &str, req: ChatRequest, stream: bool) -> Result<WebRequestData> {
		let api_key = get_api_key_from_config(self.config(), Self::default_api_key_env_name())?;
		let headers: Vec<(String, String)> = [
			// request headers (required)
			("x-api-key", api_key.as_str()),
			("anthropic-version", ANTRHOPIC_VERSION),
		]
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.into_iter()
		.collect(); // region:    --- Section

		// endregion: --- Section

		let AnthropicsRequestParts { system, messages } = into_anthropic_request_parts(req.messages)?;
		let mut payload = json!({
			"model": model,
			"max_tokens": MAX_TOKENS,
			"messages": messages,
			"stream": stream
		});
		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}
		Ok(WebRequestData { headers, payload })
	}
}
// endregion: --- Support Implementations

// region:    --- Support Functions

// endregion: --- Support Functions

/// This is the capture the various part that the Anthropic messages API expect
/// Take the Anthropic response and make it a ChatResponse
/// The body format is `content: {"text": string}[]`
/// NOTE: Since ChatResponse is for multi-provider, we have to have an intermediary
///       step, and right now, just just go from raw Value to ChatResponse
///       This makes the system more flexible, but we could change that later.
fn into_genai_chat_response(mut response: Response) -> Result<ChatResponse> {
	let json_content_items: Vec<Value> = response.body.x_take("content")?;

	let mut content: Vec<String> = Vec::new();

	for mut item in json_content_items {
		let item_text: String = item.x_take("text")?;
		content.push(item_text);
	}

	let content = if content.is_empty() {
		None
	} else {
		Some(content.join(""))
	};

	Ok(ChatResponse { content })
}

/// see: https://docs.anthropic.com/en/api/messages
struct AnthropicsRequestParts {
	system: Option<String>,
	messages: Vec<Value>,
	// TODO: need to add tools
}

/// Takes the genai ChatMessages and build the System string and json Messages for Anthropic.
/// NOTE: Here we do not use serde serialization as we might want to use the annotations for other purpose later.
fn into_anthropic_request_parts(msgs: Vec<ChatMessage>) -> Result<AnthropicsRequestParts> {
	let mut messages: Vec<Value> = Vec::new();
	let mut systems: Vec<String> = Vec::new();

	for msg in msgs {
		let content = msg.content;
		match msg.role {
			// for now, system and tool goes to system
			ChatRole::System | ChatRole::Tool => systems.push(content),
			ChatRole::User => messages.push(json! ({"role": "user", "content": content})),
			ChatRole::Assistant => messages.push(json! ({"role": "assistant", "content": content})),
		}
	}

	let system = if !systems.is_empty() {
		Some(systems.join("\n"))
	} else {
		None
	};

	Ok(AnthropicsRequestParts { system, messages })
}
