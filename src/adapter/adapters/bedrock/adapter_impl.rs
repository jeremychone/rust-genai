//! AWS Bedrock Adapter Implementation
//!
//! Implements the Adapter trait for AWS Bedrock's Converse API.
//! Uses Bearer token authentication with AWS Bedrock API keys.

use crate::adapter::adapters::bedrock::streamer::BedrockStreamer;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse,
	ContentPart, MessageContent, ToolCall, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{Value, json};
use tracing::warn;
use value_ext::JsonValueExt;

/// List of known Bedrock model IDs
pub const MODELS: &[&str] = &[
	// Anthropic Claude models
	"anthropic.claude-3-5-sonnet-20241022-v2:0",
	"anthropic.claude-3-5-haiku-20241022-v1:0",
	"anthropic.claude-3-opus-20240229-v1:0",
	"anthropic.claude-3-sonnet-20240229-v1:0",
	"anthropic.claude-3-haiku-20240307-v1:0",
	// Meta Llama models
	"meta.llama3-2-90b-instruct-v1:0",
	"meta.llama3-2-11b-instruct-v1:0",
	"meta.llama3-2-3b-instruct-v1:0",
	"meta.llama3-2-1b-instruct-v1:0",
	"meta.llama3-1-405b-instruct-v1:0",
	"meta.llama3-1-70b-instruct-v1:0",
	"meta.llama3-1-8b-instruct-v1:0",
	"meta.llama3-70b-instruct-v1:0",
	"meta.llama3-8b-instruct-v1:0",
	// Amazon Titan models
	"amazon.titan-text-premier-v1:0",
	"amazon.titan-text-express-v1",
	"amazon.titan-text-lite-v1",
	// Mistral models
	"mistral.mistral-large-2407-v1:0",
	"mistral.mistral-large-2402-v1:0",
	"mistral.mistral-small-2402-v1:0",
	"mistral.mistral-7b-instruct-v0:2",
	"mistral.mixtral-8x7b-instruct-v0:1",
	// Cohere models
	"cohere.command-r-plus-v1:0",
	"cohere.command-r-v1:0",
	// AI21 models
	"ai21.jamba-1-5-large-v1:0",
	"ai21.jamba-1-5-mini-v1:0",
];

/// Default max tokens for Bedrock models
const DEFAULT_MAX_TOKENS: u32 = 4096;

pub struct BedrockAdapter;

impl BedrockAdapter {
	/// Environment variable name for AWS Bedrock API key (Bearer token)
	pub const API_KEY_ENV: &str = "AWS_BEARER_TOKEN_BEDROCK";
	/// Environment variable for AWS region
	pub const AWS_REGION_ENV: &str = "AWS_REGION";

	/// Get the AWS region from environment or default
	fn get_region() -> String {
		std::env::var(Self::AWS_REGION_ENV).unwrap_or_else(|_| "us-east-1".to_string())
	}

	/// Build the Bedrock endpoint URL for the given region
	fn build_endpoint(region: &str) -> String {
		format!("https://bedrock-runtime.{}.amazonaws.com/", region)
	}

	/// Convert Usage from Bedrock format
	fn into_usage(mut usage_value: Value) -> Usage {
		let input_tokens: i32 = usage_value.x_take("inputTokens").ok().unwrap_or(0);
		let output_tokens: i32 = usage_value.x_take("outputTokens").ok().unwrap_or(0);
		let total_tokens = input_tokens + output_tokens;

		Usage {
			prompt_tokens: Some(input_tokens),
			prompt_tokens_details: None,
			completion_tokens: Some(output_tokens),
			completion_tokens_details: None,
			total_tokens: Some(total_tokens),
		}
	}

	/// Convert ChatRequest to Bedrock Converse API format
	fn into_bedrock_request_parts(chat_req: ChatRequest) -> Result<BedrockRequestParts> {
		let mut messages: Vec<Value> = Vec::new();
		let mut system_content: Vec<Value> = Vec::new();

		// Process system prompt
		if let Some(system) = chat_req.system {
			system_content.push(json!({"text": system}));
		}

		// Process messages
		for msg in chat_req.messages {
			match msg.role {
				ChatRole::System => {
					// Collect system messages into system content
					if let Some(text) = msg.content.joined_texts() {
						system_content.push(json!({"text": text}));
					}
				}
				ChatRole::User => {
					let content = Self::convert_content_parts_to_bedrock(msg.content, true)?;
					if !content.is_empty() {
						messages.push(json!({
							"role": "user",
							"content": content
						}));
					}
				}
				ChatRole::Assistant => {
					let content = Self::convert_content_parts_to_bedrock(msg.content, false)?;
					if !content.is_empty() {
						messages.push(json!({
							"role": "assistant",
							"content": content
						}));
					}
				}
				ChatRole::Tool => {
					// Tool responses in Bedrock go as user messages with toolResult
					let mut tool_results: Vec<Value> = Vec::new();
					for part in msg.content {
						if let ContentPart::ToolResponse(tool_response) = part {
							tool_results.push(json!({
								"toolResult": {
									"toolUseId": tool_response.call_id,
									"content": [{"text": tool_response.content}]
								}
							}));
						}
					}
					if !tool_results.is_empty() {
						messages.push(json!({
							"role": "user",
							"content": tool_results
						}));
					}
				}
			}
		}

		// Convert tools to Bedrock format
		let tool_config = chat_req.tools.map(|tools| {
			let tool_specs: Vec<Value> = tools
				.into_iter()
				.map(|tool| {
					let mut spec = json!({
						"toolSpec": {
							"name": tool.name,
							"inputSchema": {
								"json": tool.schema
							}
						}
					});
					if let Some(desc) = tool.description {
						let _ = spec.x_insert("/toolSpec/description", desc);
					}
					spec
				})
				.collect();
			json!({"tools": tool_specs})
		});

		Ok(BedrockRequestParts {
			system: if system_content.is_empty() {
				None
			} else {
				Some(system_content)
			},
			messages,
			tool_config,
		})
	}

	/// Convert GenAI content parts to Bedrock content format
	fn convert_content_parts_to_bedrock(content: MessageContent, is_user: bool) -> Result<Vec<Value>> {
		let mut parts: Vec<Value> = Vec::new();

		for part in content {
			match part {
				ContentPart::Text(text) => {
					parts.push(json!({"text": text}));
				}
				ContentPart::Binary(binary) if is_user => {
					let is_image = binary.is_image();
					let Binary {
						content_type, source, ..
					} = binary;

					if is_image {
						match source {
							BinarySource::Base64(data) => {
								// Extract format from content type (e.g., "image/png" -> "png")
								let format = content_type.split('/').nth(1).unwrap_or("png").to_string();
								parts.push(json!({
									"image": {
										"format": format,
										"source": {
											"bytes": data
										}
									}
								}));
							}
							BinarySource::Url(_) => {
								warn!("Bedrock doesn't support images from URL directly");
							}
						}
					} else {
						// Document handling
						match source {
							BinarySource::Base64(data) => {
								let format = content_type.split('/').nth(1).unwrap_or("pdf").to_string();
								parts.push(json!({
									"document": {
										"format": format,
										"source": {
											"bytes": data
										}
									}
								}));
							}
							BinarySource::Url(_) => {
								warn!("Bedrock doesn't support documents from URL directly");
							}
						}
					}
				}
				ContentPart::ToolCall(tool_call) if !is_user => {
					parts.push(json!({
						"toolUse": {
							"toolUseId": tool_call.call_id,
							"name": tool_call.fn_name,
							"input": tool_call.fn_arguments
						}
					}));
				}
				ContentPart::ToolResponse(tool_response) if is_user => {
					parts.push(json!({
						"toolResult": {
							"toolUseId": tool_response.call_id,
							"content": [{"text": tool_response.content}]
						}
					}));
				}
				_ => {
					// Skip unsupported content types for the given role
				}
			}
		}

		Ok(parts)
	}
}

impl Adapter for BedrockAdapter {
	fn default_endpoint() -> Endpoint {
		let region = Self::get_region();
		Endpoint::from_owned(Self::build_endpoint(&region))
	}

	fn default_auth() -> AuthData {
		// Bedrock uses Bearer token authentication
		AuthData::from_env(Self::API_KEY_ENV)
	}

	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let (model_name, _) = model.model_name.as_model_name_and_namespace();

		// URL encode the model ID (Bedrock model IDs contain colons)
		let encoded_model = urlencoding_encode(model_name);

		let url = match service_type {
			ServiceType::Chat => format!("{base_url}model/{encoded_model}/converse"),
			ServiceType::ChatStream => format!("{base_url}model/{encoded_model}/converse-stream"),
			ServiceType::Embed => {
				return Err(Error::AdapterNotSupported {
					adapter_kind: AdapterKind::Bedrock,
					feature: "embeddings".to_string(),
				});
			}
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

		// Get API key (Bearer token)
		let api_key = get_api_key(auth, &model)?;

		// Get the URL
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// Build headers with Bearer token authentication
		let headers = Headers::from(vec![
			("Authorization".to_string(), format!("Bearer {}", api_key)),
			("Content-Type".to_string(), "application/json".to_string()),
		]);

		// Convert chat request to Bedrock format
		let BedrockRequestParts {
			system,
			messages,
			tool_config,
		} = Self::into_bedrock_request_parts(chat_req)?;

		// Build the payload
		let mut payload = json!({
			"messages": messages
		});

		// Add system prompt if present
		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}

		// Add tool config if present
		if let Some(tool_config) = tool_config {
			payload.x_insert("toolConfig", tool_config)?;
		}

		// Build inference configuration
		let mut inference_config = json!({});

		// Max tokens (required for most Bedrock models)
		let max_tokens = options_set.max_tokens().unwrap_or(DEFAULT_MAX_TOKENS);
		inference_config.x_insert("maxTokens", max_tokens)?;

		// Temperature
		if let Some(temperature) = options_set.temperature() {
			// Bedrock expects temperature between 0 and 1
			let clamped_temp = temperature.clamp(0.0, 1.0);
			if (temperature - clamped_temp).abs() > f64::EPSILON {
				warn!("Temperature {} clamped to {} for Bedrock", temperature, clamped_temp);
			}
			inference_config.x_insert("temperature", clamped_temp)?;
		}

		// Top P
		if let Some(top_p) = options_set.top_p() {
			inference_config.x_insert("topP", top_p)?;
		}

		// Stop sequences
		if !options_set.stop_sequences().is_empty() {
			inference_config.x_insert("stopSequences", options_set.stop_sequences())?;
		}

		payload.x_insert("inferenceConfig", inference_config)?;

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		let captured_raw_body = options_set.capture_raw_body().unwrap_or_default().then(|| body.clone());

		// Extract usage
		let usage = body.x_take::<Value>("usage");
		let usage = usage.map(Self::into_usage).unwrap_or_default();

		// Extract provider model name from stopReason metadata if available
		let provider_model_iden = model_iden.clone();

		// Process output message
		let mut content: MessageContent = MessageContent::default();
		let mut reasoning_content: Option<String> = None;

		if let Ok(output) = body.x_take::<Value>("output") {
			if let Ok(message) = output.x_get::<Value>("message") {
				if let Ok(content_blocks) = message.x_get::<Vec<Value>>("content") {
					for mut block in content_blocks {
						// Text content
						if let Ok(text) = block.x_take::<String>("text") {
							content.push(ContentPart::Text(text));
						}
						// Tool use content
						else if let Ok(mut tool_use) = block.x_take::<Value>("toolUse") {
							let call_id = tool_use.x_take::<String>("toolUseId")?;
							let fn_name = tool_use.x_take::<String>("name")?;
							let fn_arguments = tool_use.x_take::<Value>("input").unwrap_or(Value::Null);

							content.push(ContentPart::ToolCall(ToolCall {
								call_id,
								fn_name,
								fn_arguments,
							}));
						}
						// Reasoning/thinking content (if supported by model)
						else if let Ok(thinking) = block.x_take::<String>("reasoningContent") {
							reasoning_content = Some(thinking);
						}
					}
				}
			}
		}

		Ok(ChatResponse {
			content,
			reasoning_content,
			model_iden,
			provider_model_iden,
			usage,
			captured_raw_body,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSource::new(reqwest_builder)?;
		let bedrock_stream = BedrockStreamer::new(event_source, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(bedrock_stream);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::Bedrock,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::Bedrock,
			feature: "embeddings".to_string(),
		})
	}
}

/// Helper struct for Bedrock request parts
struct BedrockRequestParts {
	system: Option<Vec<Value>>,
	messages: Vec<Value>,
	tool_config: Option<Value>,
}

/// Simple URL encoding for model IDs
fn urlencoding_encode(s: &str) -> String {
	let mut result = String::new();
	for byte in s.bytes() {
		match byte {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
				result.push(byte as char);
			}
			_ => {
				result.push_str(&format!("%{:02X}", byte));
			}
		}
	}
	result
}
