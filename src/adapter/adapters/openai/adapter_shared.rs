//! This is support implementation of the OpenAI Adapter which can also be called by other OpenAI Adapter Variants

use super::cache_policy::{OpenAiPromptCachePolicy, OpenAiProtocol, is_gpt_5_6_or_later, openai_prompt_cache_policy};
use super::schema::{OpenAiResponseFormatPlan, response_format_plan, tool_parameters_schema};
use crate::adapter::adapters::openai::OpenAIAdapter;
use crate::adapter::adapters::support::{TOOL_RESULT_IMAGES_LABEL, get_api_key, tool_response_fallback_text};
use crate::adapter::{AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	BinarySource, CacheControl, ChatOptionsSet, ChatRequest, ChatRole, ContentPart, ReasoningEffort, ToolChoice,
	ToolResponse, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebClient;
use crate::{Error, Headers, Result};
use crate::{ModelIden, ServiceTarget};
use serde_json::{Value, json};
use tracing::error;
use tracing::warn;
use value_ext::JsonValueExt;

fn insert_openai_reasoning_effort(payload: &mut Value, effort: &ReasoningEffort) -> Result<()> {
	let keyword = match effort {
		ReasoningEffort::Zero => "none",
		ReasoningEffort::Low => "low",
		ReasoningEffort::Medium => "medium",
		ReasoningEffort::High => "high",
		ReasoningEffort::XHigh => "xhigh",
		ReasoningEffort::Max => "max",
		ReasoningEffort::Minimal => "minimal",
		ReasoningEffort::Budget(_) => return Ok(()),
	};

	payload.x_insert("reasoning_effort", keyword)?;

	Ok(())
}

fn openai_tool_choice(tool_choice: Option<&ToolChoice>) -> Option<Value> {
	match tool_choice? {
		ToolChoice::Auto => Some(json!("auto")),
		ToolChoice::None => Some(json!("none")),
		ToolChoice::Required => Some(json!("required")),
		ToolChoice::Tool { name } => Some(json!({
			"type": "function",
			"function": { "name": name }
		})),
	}
}

/// Support functions for other adapters that share OpenAI APIs
impl OpenAIAdapter {
	pub(in crate::adapter::adapters) fn util_get_service_url(
		_model: &ModelIden,
		service_type: ServiceType,
		// -- utility arguments
		default_endpoint: Endpoint,
	) -> Result<String> {
		let base_url = default_endpoint.base_url();
		// Parse into URL and query-params
		let base_url = reqwest::Url::parse(base_url)
			.map_err(|err| Error::Internal(format!("Cannot parse url: {base_url}. Cause:\n{err}")))?;
		let original_query_params = base_url.query().to_owned();

		let suffix = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => "chat/completions",
			ServiceType::Embed => "embeddings",
		};
		let mut full_url = base_url.join(suffix).map_err(|err| {
			Error::Internal(format!(
				"Cannot join suffix '{suffix}' for url: {base_url}. Cause:\n{err}"
			))
		})?;
		full_url.set_query(original_query_params);
		Ok(full_url.to_string())
	}

	/// Shared OpenAI to_web_request_data for various OpenAI compatible adapters
	/// NOTE: `messages` is inserted after tool fields to improve prompt-cache utilization.
	///        See PR 262: https://github.com/jeremychone/rust-genai/pull/262
	pub(in crate::adapter::adapters) fn util_to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
		custom: Option<ToWebRequestDataOptions>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = target;
		let (_, model_name) = model.model_name.namespace_and_name();
		let protocol = OpenAiProtocol::ChatCompletions;
		let prompt_cache_policy =
			openai_prompt_cache_policy(model.adapter_kind, model_name, &chat_req, &options_set, protocol);
		let response_format_plan = response_format_plan(&options_set);

		// -- url
		let url = AdapterDispatcher::get_service_url(&model, service_type, endpoint)?;

		// -- api_key / headers
		// NOTE: useful for local providers
		let allow_anonymous = matches!(auth, AuthData::None) && custom.as_ref().is_some_and(|c| c.allow_no_api_key);

		let headers = if !allow_anonymous {
			let api_key = get_api_key(auth, &model)?;
			Headers::from(("Authorization".to_string(), format!("Bearer {api_key}")))
		} else {
			Headers::default()
		};

		let stream = matches!(service_type, ServiceType::ChatStream);
		let managed_body_thinking = custom.as_ref().is_some_and(|custom| custom.managed_body_thinking);

		// -- compute reasoning_effort and eventual trimmed model_name
		// For now, just for openai AdapterKind
		let (reasoning_effort, model_name): (Option<ReasoningEffort>, &str) = {
			let (reasoning_effort, model_name) = options_set
				.reasoning_effort()
				.cloned()
				.map(|v| (Some(v), model_name))
				.unwrap_or_else(|| ReasoningEffort::from_model_name(model_name));

			(reasoning_effort, model_name)
		};

		// -- Build the basic payload

		let OpenAIRequestParts { messages, tools } =
			Self::into_openai_request_parts(&model, chat_req, prompt_cache_policy.as_ref())?;
		let mut payload = json!({
			"model": model_name,
			"stream": stream
		});

		if let Some(policy) = prompt_cache_policy.as_ref() {
			let mut prompt_cache_options = json!({"mode": "explicit"});
			if let Some(ttl) = policy.ttl {
				prompt_cache_options["ttl"] = json!(ttl);
			}
			payload.x_insert("prompt_cache_options", prompt_cache_options)?;
		}

		// -- Set reasoning effort
		if let Some(reasoning_effort) = reasoning_effort.as_ref() {
			if managed_body_thinking {
				let thinking_type = if matches!(reasoning_effort, ReasoningEffort::Zero) {
					"disabled"
				} else {
					"enabled"
				};
				payload.x_insert("thinking", json!({"type": thinking_type}))?;

				if !matches!(reasoning_effort, ReasoningEffort::Zero) {
					insert_openai_reasoning_effort(&mut payload, reasoning_effort)?;
				}
			} else {
				insert_openai_reasoning_effort(&mut payload, reasoning_effort)?;
			}
		}

		// -- Set verbosity
		if let Some(verbosity) = options_set.verbosity()
			&& let Some(keyword) = verbosity.as_keyword()
		{
			payload.x_insert("verbosity", keyword)?;
		}

		// -- Tools (before messages)
		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}
		if let Some(tool_choice) = openai_tool_choice(options_set.tool_choice()) {
			payload.x_insert("tool_choice", tool_choice)?;
		}

		// -- Messages (after tools)
		payload.x_insert("messages", messages)?;

		// -- Add options
		let response_format = match response_format_plan {
			OpenAiResponseFormatPlan::None => None,
			OpenAiResponseFormatPlan::JsonMode => Some(json!({"type": "json_object"})),
			OpenAiResponseFormatPlan::JsonSchema { name, schema } => Some(json!({
				"type": "json_schema",
				"json_schema": {
					"name": name,
					"strict": true,
					"schema": schema,
				}
			})),
		};

		if let Some(response_format) = response_format {
			payload["response_format"] = response_format;
		}

		// -- Add supported ChatOptions
		if stream & options_set.capture_usage().unwrap_or(false) {
			payload.x_insert("stream_options", json!({"include_usage": true}))?;
		}

		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop", options_set.stop_sequences())?;
		}

		// GPT-5.x and o-series models require "max_completion_tokens" instead of "max_tokens"
		let max_tokens_key = if model_name.starts_with("gpt-5")
			|| model_name.starts_with("o1")
			|| model_name.starts_with("o3")
			|| model_name.starts_with("o4")
		{
			"max_completion_tokens"
		} else {
			"max_tokens"
		};
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert(max_tokens_key, max_tokens)?;
		} else if let Some(custom) = custom.as_ref()
			&& let Some(max_tokens) = custom.default_max_tokens
		{
			payload.x_insert(max_tokens_key, max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}
		if let Some(seed) = options_set.seed() {
			payload.x_insert("seed", seed)?;
		}
		if let Some(service_tier) = options_set.service_tier()
			&& let Some(keyword) = service_tier.as_keyword()
		{
			payload.x_insert("service_tier", keyword)?;
		}

		// -- OpenAI prompt cache options
		if let Some(prompt_cache_key) = options_set.prompt_cache_key() {
			payload.x_insert("prompt_cache_key", prompt_cache_key)?;
		}
		if !is_gpt_5_6_or_later(model_name)
			&& let Some(cache_control) = options_set.cache_control()
		{
			let prompt_cache_retention = match cache_control {
				CacheControl::Memory | CacheControl::Ephemeral => Some("in_memory"),
				CacheControl::Ephemeral24h => Some("24h"),
				CacheControl::Ephemeral5m | CacheControl::Ephemeral1h => None,
			};
			if let Some(prompt_cache_retention) = prompt_cache_retention {
				payload.x_insert("prompt_cache_retention", prompt_cache_retention)?;
			}
		}

		// -- Provider-specific payload extension
		// Merged last so callers can intentionally override previously set fields.
		if let Some(extra_body) = options_set.extra_body() {
			payload.x_merge(extra_body.clone())?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	/// Note: Needs to be called from super::streamer as well
	pub(super) fn into_usage(adapter: AdapterKind, usage_value: Value) -> Usage {
		if usage_value.is_null() {
			return Usage::default();
		}

		// NOTE: here we make sure we do not fail since we do not want to break a response because usage parsing fail
		let usage = serde_json::from_value(usage_value).map_err(|err| {
			error!("Fail to deserialize usage. Cause: {err}");
			err
		});
		let mut usage: Usage = usage.unwrap_or_default();
		// Will set details to None if no values
		usage.compact_details();

		// Unfortunately, xAI grok-3 does not compute reasoning tokens correctly.
		// Example: completion_tokens: 35, completion_tokens_details.reasoning_tokens: 192
		// BUT completion_tokens should be 35 + 192.
		// TODO: We might want to do this for other token details as well.
		// TODO: We could check if the math adds up first with the total token count, and only change it if it does not.
		//       This will allow us to be forward compatible if/when they fix this bug (yes, it is a bug).
		if matches!(adapter, AdapterKind::Xai)
			&& let Some(reasoning_tokens) = usage.completion_tokens_details.as_ref().and_then(|d| d.reasoning_tokens)
		{
			let completion_tokens = usage.completion_tokens.unwrap_or(0);
			usage.completion_tokens = Some(completion_tokens + reasoning_tokens)
		}

		usage
	}

	/// Takes the genai ChatMessages and builds the OpenAIChatRequestParts
	/// - `genai::ChatRequest.system`, if present, is added as the first message with role 'system'.
	/// - All messages get added with the corresponding roles (tools are not supported for now)
	fn into_openai_request_parts(
		model_iden: &ModelIden,
		chat_req: ChatRequest,
		cache_policy: Option<&OpenAiPromptCachePolicy>,
	) -> Result<OpenAIRequestParts> {
		let mut messages: Vec<Value> = Vec::new();

		// -- Process the system
		if let Some(system_msg) = chat_req.system {
			messages.push(json!({"role": "system", "content": system_msg}));
		}

		// Images attached to tool responses (`ToolResponse.parts`) cannot ride inside a
		// Chat Completions `tool` message, so they are carried by a follow-up `user`
		// message. Images from a run of consecutive Tool messages are batched into one
		// trailing user message, emitted before the next non-tool message.
		let mut pending_tool_images: Vec<Value> = Vec::new();

		// -- Process the messages
		for msg in chat_req.messages {
			if !matches!(msg.role, ChatRole::Tool) && !pending_tool_images.is_empty() {
				messages.push(tool_images_user_message(std::mem::take(&mut pending_tool_images)));
			}

			let cache_controlled = cache_policy.is_some()
				&& msg
					.options
					.as_ref()
					.and_then(|options| options.cache_control.as_ref())
					.is_some();

			// Note: Will handle more types later
			match msg.role {
				// For now, system and tool messages go to the system
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						if cache_controlled {
							let mut values = vec![json!({"type": "text", "text": content})];
							apply_chat_cache_breakpoint(model_iden, &mut values, "message")?;
							messages.push(json!({"role": "system", "content": values}));
						} else {
							messages.push(json!({"role": "system", "content": content}))
						}
					}
					// TODO: Probably need to warn if it is a ToolCalls type of content
				}

				// User - For now support Text and Binary
				ChatRole::User => {
					// -- If we have only text, then, we jjust returned the joined_texts
					if msg.content.is_text_only() && !cache_controlled {
						// NOTE: for now, if no content, just return empty string (respect current logic)
						let content = json!(msg.content.joined_texts().unwrap_or_else(String::new));
						messages.push(json! ({"role": "user", "content": content}));
					} else {
						let mut values: Vec<Value> = Vec::new();
						for part in msg.content {
							match part {
								ContentPart::Text(content) => values.push(json!({"type": "text", "text": content})),
								ContentPart::Binary(binary) => {
									let is_audio = binary.is_audio();
									let is_image = binary.is_image();

									// let Binary {
									// 	content_type, source, ..
									// } = binary;

									if is_audio {
										match &binary.source {
											BinarySource::Url(_url) => {
												warn!(
													"OpenAI doesn't support audio from URL, need to handle it gracefully"
												);
											}
											BinarySource::Base64(content) => {
												let mut format =
													binary.content_type.split('/').next_back().unwrap_or("");
												if format == "mpeg" {
													format = "mp3";
												}
												values.push(json!({
													"type": "input_audio",
													"input_audio": {
														"data": content,
														"format": format
													}
												}));
											}
										}
									} else if is_image {
										let image_url = binary.into_url();
										values.push(json!({"type": "image_url", "image_url": {"url": image_url}}));
									} else if binary.is_video() {
										// OpenAI-compatible providers that support video (e.g. Alibaba qwen)
										// accept it as a `video_url` content part, symmetric to `image_url`.
										let video_url = binary.into_url();
										values.push(json!({"type": "video_url", "video_url": {"url": video_url}}));
									} else if matches!(&binary.source, BinarySource::Url(_)) {
										// TODO: Need to return error
										warn!("OpenAI doesn't support file from URL, need to handle it gracefully");
									} else {
										let filename = binary.name.clone();
										let file_base64_url = binary.into_url();
										values.push(json!({"type": "file", "file": {
											"filename": filename,
											"file_data": file_base64_url
										}}))
									}
								}

								// Use `match` instead of `if let`. This will allow to future-proof this
								// implementation in case some new message content types would appear,
								// this way library would not compile if not all methods are implemented
								// continue would allow to gracefully skip pushing unserializable message
								// TODO: Probably need to warn if it is a ToolCalls type of content
								ContentPart::ToolCall(_) => (),
								ContentPart::ToolResponse(_) => (),
								ContentPart::ThoughtSignature(_) => (),
								ContentPart::ReasoningContent(_) => (),
								// Custom are ignored for this logic
								ContentPart::Custom(_) => {}
							}
						}
						if cache_controlled {
							apply_chat_cache_breakpoint(model_iden, &mut values, "message")?;
						}
						messages.push(json! ({"role": "user", "content": values}));
					}
				}

				// Assistant - For now support Text and ToolCalls
				ChatRole::Assistant => {
					let mut texts: Vec<String> = Vec::new();
					let mut tool_calls: Vec<Value> = Vec::new();
					let mut reasoning_parts: Vec<String> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::Text(text) => texts.push(text),
							ContentPart::ToolCall(tool_call) => {
								//
								tool_calls.push(json!({
									"type": "function",
									"id": tool_call.call_id,
									"function": {
										"name": tool_call.fn_name,
										"arguments": tool_call.fn_arguments.to_string(),
									}
								}))
							}
							// Extract reasoning content parts to hoist into sibling field
							ContentPart::ReasoningContent(reasoning) => reasoning_parts.push(reasoning),

							// TODO: Probably need towarn on this one (probably need to add binary here)
							ContentPart::Binary(_) => (),
							ContentPart::ToolResponse(_) => (),
							ContentPart::ThoughtSignature(_) => {}
							// Custom are ignored for this logic
							ContentPart::Custom(_) => {}
						}
					}
					let mut message = if cache_controlled {
						let mut values = texts
							.into_iter()
							.map(|text| json!({"type": "text", "text": text}))
							.collect::<Vec<Value>>();
						apply_chat_cache_breakpoint(model_iden, &mut values, "message")?;
						json!({"role": "assistant", "content": values})
					} else {
						let content = texts.join("\n\n");
						json!({"role": "assistant", "content": content})
					};
					if !tool_calls.is_empty() {
						message.x_insert("tool_calls", tool_calls)?;
					}
					// Echo reasoning_content back for providers that require it (Kimi, DeepSeek)
					// Note: In practice there is at most one ReasoningContent part per message,
					//       but we join defensively in case multiple parts are present.
					if !reasoning_parts.is_empty() {
						message.x_insert("reasoning_content", reasoning_parts.join("\n"))?;
					}
					messages.push(message);
				}

				// Tool - For now, support only tool responses
				ChatRole::Tool => {
					for part in msg.content {
						if let ContentPart::ToolResponse(tool_response) = part {
							let ToolResponse {
								call_id,
								content,
								parts,
								..
							} = tool_response;
							let parts = parts.unwrap_or_default();

							if parts.is_empty() {
								messages.push(json!({
									"role": "tool",
									"content": content,
									"tool_call_id": call_id,
								}));
							} else {
								let mut image_values: Vec<Value> = Vec::new();
								for binary in parts {
									if binary.is_image() {
										let image_url = binary.into_url();
										image_values
											.push(json!({"type": "image_url", "image_url": {"url": image_url}}));
									} else {
										warn!(
											"ToolResponse binary parts only support images for OpenAI-compatible adapters; skipping non-image part '{}'",
											binary.content_type
										);
									}
								}
								let content = tool_response_fallback_text(content, !image_values.is_empty());
								messages.push(json!({
									"role": "tool",
									"content": content,
									"tool_call_id": call_id,
								}));
								pending_tool_images.extend(image_values);
							}
						}
					}

					// TODO: Probably need to trace/warn that this will be ignored
				}
			}
		}

		// Flush tool-result images from a trailing run of Tool messages.
		if !pending_tool_images.is_empty() {
			messages.push(tool_images_user_message(pending_tool_images));
		}

		// -- Process the tools
		let tools = chat_req.tools.map(|tools| {
			tools
				.into_iter()
				.map(|tool| {
					let strict = tool.strict.unwrap_or(false);
					let parameters = tool_parameters_schema(tool.schema, strict);

					json!({
						"type": "function",
						"function": {
							"name": tool.name,
							"description": tool.description,
							"parameters": parameters,
							"strict": strict,
						}
					})
				})
				.collect::<Vec<Value>>()
		});

		Ok(OpenAIRequestParts { messages, tools })
	}

	pub(in crate::adapter::adapters) async fn list_model_names_for_end_target(
		kind: AdapterKind,
		endpoint: Endpoint,
		auth: AuthData,
		web_client: &WebClient,
	) -> Result<Vec<String>> {
		// -- url
		let base_url = endpoint.base_url();
		let url = format!("{base_url}models");

		// -- auth / headers
		// NOTE: In this case, we accept it if the API key is not defined, and let the provider complain.
		//       This is compared to web request data that requires it before the request, except if the options say otherwise.
		//       Will need to align at some point.
		let api_key = auth.single_key_value().ok();
		let headers = api_key
			.map(|api_key| Headers::from(("Authorization".to_string(), format!("Bearer {api_key}"))))
			.unwrap_or_default();

		// -- Exec request
		let mut res = web_client
			.do_get(&url, &headers)
			.await
			.map_err(|webc_error| Error::WebAdapterCall {
				adapter_kind: kind,
				webc_error,
			})?;

		// -- Format result
		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("data")? {
			for mut model in models_value {
				let model_name: String = model.x_take("id")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}
}

/// Custom OpenAI structure for Adapters to use to customize
/// the default [`OpenAIAdapter::util_to_web_request_data`]
///
/// These options are supplied by adapter implementations to describe
/// provider-specific request-body behavior. They are not general chat
/// settings, and each field is opt-in.
#[derive(Default)]
pub struct ToWebRequestDataOptions {
	/// Optional fallback for providers that requires max tokens.
	///
	/// The Fireworks adapter uses this only when no effective `max_tokens` option
	/// was supplied. Other adapter paths leave this fallback unset.
	pub default_max_tokens: Option<u32>,

	/// Allows a provider request to be built without an API key.
	///
	/// When this is true and authentication is `AuthData::None`, the shared
	/// builder omits the `Authorization` header instead of requiring a key.
	/// This is intended for local or otherwise anonymous OpenAI-compatible
	/// endpoints. It affects only adapter paths that explicitly enable it and
	/// remains false for normal remote providers.
	pub allow_no_api_key: bool,

	/// Enables provider-specific `thinking.type` serialization coordinated with `reasoning_effort`.
	///
	/// `DeepSeekAdapter` is currently the only adapter that opts in. Its
	/// OpenAI-compatible endpoint receives `thinking.type = "disabled"` for
	/// explicit zero effort, or `"enabled"` for non-zero effort, while the
	/// existing reasoning-effort serialization is retained where supported.
	/// `Budget(_)` continues to follow the existing serializer behavior and
	/// does not become a keyword value. Other OpenAI-compatible adapters leave
	/// this false, so their payloads do not gain a `thinking` field.
	pub managed_body_thinking: bool,
}

// region:    --- Support

struct OpenAIRequestParts {
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

/// Build the follow-up `user` message that carries tool-result images
/// (`ToolResponse.parts`), since Chat Completions `tool` message content
/// cannot include image blocks.
fn tool_images_user_message(image_values: Vec<Value>) -> Value {
	let mut values: Vec<Value> = Vec::with_capacity(image_values.len() + 1);
	values.push(json!({"type": "text", "text": TOOL_RESULT_IMAGES_LABEL}));
	values.extend(image_values);
	json!({"role": "user", "content": values})
}

fn apply_chat_cache_breakpoint(_model_iden: &ModelIden, content: &mut [Value], _scope: &'static str) -> Result<()> {
	let Some(content_block) = content.iter_mut().rev().find(|value| {
		matches!(
			value.get("type").and_then(Value::as_str),
			Some("text" | "image_url" | "input_audio" | "file" | "refusal")
		)
	}) else {
		return Ok(());
	};

	content_block.x_insert("prompt_cache_breakpoint", json!({"mode": "explicit"}))?;
	Ok(())
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "adapter_shared_tests.rs"]
// The shared builder test uses neutral model names because this checks the
// shared reasoning suffix parser, not GPT-specific request behavior.
mod tests;

// endregion: --- Tests
