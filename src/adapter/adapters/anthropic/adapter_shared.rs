use super::AnthropicAdapter;
use super::ant_model::{AnthropicMaxTokens, AnthropicModel, AnthropicModelCapabilities};
use crate::Result;
use crate::adapter::adapters::anthropic::ant_reasoning::insert_anthropic_reasoning;
use crate::adapter::adapters::support::{assistant_embedded_tool_response_err, get_api_key};
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, CacheControl, CacheCreationDetails, ChatOptionsSet, ChatRequest, ChatResponse,
	ChatResponseFormat, ChatRole, ContentPart, JsonSchemaDialect, MessageContent, PromptTokensDetails, ReasoningEffort,
	StopReason, Tool, ToolCall, ToolChoice, ToolConfig, ToolName, ToolResponse, Usage, sanitize_json_schema,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebClient, WebResponse};
use crate::{Headers, ModelIden};
use serde_json::{Map, Value, json};
use tracing::warn;
use value_ext::JsonValueExt;

const ANTHROPIC_VERSION: &str = "2023-06-01";

// NOTE: For Anthropic, the max_tokens must be specified.
//       To avoid surprises, the default value for genai is the maximum for a given model.
// Current logic:
// - if model contains `3-opus` or `3-haiku` 4x max token limit,
// - otherwise assume 8k model
//
// NOTE: Will need to add the thinking option: https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking
// For max model tokens see: https://docs.anthropic.com/en/docs/about-claude/models/overview
//
// fall back
pub(in crate::adapter::adapters) const MAX_TOKENS_64K: u32 = 64000; // claude-opus-4-5 claude-sonnet... (4 and above), claude-haiku..., claude-3-7-sonnet,
// custom
pub(in crate::adapter::adapters) const MAX_TOKENS_128K: u32 = 128000; // claude-opus-4-8 fable mythos
pub(in crate::adapter::adapters) const MAX_TOKENS_32K: u32 = 32000; // claude-opus-4
pub(in crate::adapter::adapters) const MAX_TOKENS_8K: u32 = 8192; // claude-3-5-sonnet, claude-3-5-haiku
pub(in crate::adapter::adapters) const MAX_TOKENS_4K: u32 = 4096; // claude-3-opus, claude-3-haiku

/// Shared Antropic Methods
impl AnthropicAdapter {
	/// Resolves the max_tokens value for an Anthropic model, using the user-provided
	/// value if set, or a model-appropriate default.
	pub(in crate::adapter::adapters) fn resolve_max_tokens(model_name: &str, options_set: &ChatOptionsSet) -> u32 {
		let capabilities = AnthropicModel::parse(model_name).capabilities();
		Self::resolve_max_tokens_for_capabilities(&capabilities, options_set)
	}

	fn resolve_max_tokens_for_capabilities(
		capabilities: &AnthropicModelCapabilities,
		options_set: &ChatOptionsSet,
	) -> u32 {
		options_set.max_tokens().unwrap_or({
			// most likely models used, so put first. Also a little wider with `claude-sonnet` (since name from version 4)
			match capabilities.max_tokens {
				// TODO: Opus 4.8 should be here as well
				AnthropicMaxTokens::Tokens128K => MAX_TOKENS_128K,
				AnthropicMaxTokens::Tokens64K => MAX_TOKENS_64K,
				AnthropicMaxTokens::Tokens32K => MAX_TOKENS_32K,
				AnthropicMaxTokens::Tokens8K => MAX_TOKENS_8K,
				AnthropicMaxTokens::Tokens4K => MAX_TOKENS_4K,
			}
			// for now, fall back on the 64K by default (might want to be more conservative)
		})
	}

	pub(in crate::adapter::adapters) fn into_usage(mut usage_value: Value) -> Usage {
		// IMPORTANT: For Anthropic, the `input_tokens` does not include `cache_creation_input_tokens` or `cache_read_input_tokens`.
		// Therefore, it must be normalized in the OpenAI style, where it includes both cached and written tokens (for symmetry).
		let input_tokens: i32 = usage_value.x_take("input_tokens").ok().unwrap_or(0);
		let cache_creation_input_tokens: i32 = usage_value.x_take("cache_creation_input_tokens").unwrap_or(0);
		let cache_read_input_tokens: i32 = usage_value.x_take("cache_read_input_tokens").unwrap_or(0);
		let completion_tokens: i32 = usage_value.x_take("output_tokens").ok().unwrap_or(0);

		// Parse cache_creation breakdown if present (TTL-specific breakdown)
		let cache_creation_details = usage_value.get("cache_creation").and_then(parse_cache_creation_details);

		// compute the prompt_tokens
		let prompt_tokens = input_tokens + cache_creation_input_tokens + cache_read_input_tokens;

		// Compute total_tokens
		let total_tokens = prompt_tokens + completion_tokens;

		// For now the logic is to have a Some of PromptTokensDetails if at least one of those value is not 0
		// TODO: Needs to be normalized across adapters.
		let prompt_tokens_details =
			if cache_creation_input_tokens > 0 || cache_read_input_tokens > 0 || cache_creation_details.is_some() {
				Some(PromptTokensDetails {
					cache_creation_tokens: Some(cache_creation_input_tokens),
					cache_creation_details,
					cached_tokens: Some(cache_read_input_tokens),
					audio_tokens: None,
				})
			} else {
				None
			};

		Usage {
			prompt_tokens: Some(prompt_tokens),
			prompt_tokens_details,

			completion_tokens: Some(completion_tokens),
			// for now, None for Anthropic
			completion_tokens_details: None,

			total_tokens: Some(total_tokens),
		}
	}

	/// Takes the GenAI ChatMessages and constructs the System string and JSON Messages for Anthropic.
	/// - Will push the `ChatRequest.system` and system message to `AnthropicRequestParts.system`
	pub(in crate::adapter::adapters) fn into_anthropic_request_parts(
		model_iden: &ModelIden,
		mut chat_req: ChatRequest,
		request_cache_control: Option<CacheControl>,
	) -> Result<AnthropicRequestParts> {
		let mut messages: Vec<Value> = Vec::new();
		// (content, cache_control)
		let mut systems: Vec<(String, Option<CacheControl>)> = Vec::new();

		// Track TTL ordering for validation (1h must come before 5m)
		let mut seen_5m_cache = false;

		// NOTE: For now, this means the first System cannot have a cache control
		//       so that we do not change too much.
		if let Some(system) = chat_req.system {
			systems.push((system, None));
		}

		// Track explicit message-level breakpoints so request-level cache_control can defer to them.
		let mut has_msg_cache = false;

		// -- Process the messages
		for msg in chat_req.messages {
			let cache_control = msg.options.and_then(|o| o.cache_control);
			// Any explicit message cache_control (incl. System-role, part of the static prefix)
			// counts as a breakpoint; request-level placement (below) defers to it.
			if cache_control.is_some() {
				has_msg_cache = true;
			}

			// Check TTL ordering constraint
			if let Some(ref cc) = cache_control {
				match cc {
					CacheControl::Memory | CacheControl::Ephemeral | CacheControl::Ephemeral5m => {
						seen_5m_cache = true;
					}
					CacheControl::Ephemeral1h | CacheControl::Ephemeral24h => {
						if seen_5m_cache {
							warn!(
								"Anthropic cache TTL ordering violation: a longer-TTL entry (Ephemeral1h/Ephemeral24h) appears after Ephemeral/Ephemeral5m. \
								Longer-TTL cache entries must appear before shorter (5-minute) entries. \
								See: https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching#mixing-different-ttls"
							);
						}
					}
				}
			}

			match msg.role {
				// Collect only text for system; other content parts are ignored by Anthropic here.
				ChatRole::System => {
					if let Some(system_text) = msg.content.joined_texts() {
						systems.push((system_text, cache_control));
					}
				}

				// User message: text, binary (image/document), and tool_result supported.
				ChatRole::User => {
					if msg.content.is_text_only() {
						let text = msg.content.joined_texts().unwrap_or_else(String::new);
						let content = apply_cache_control_to_text(cache_control.as_ref(), text);
						messages.push(json!({"role": "user", "content": content}));
					} else {
						let mut values: Vec<Value> = Vec::new();
						for part in msg.content {
							match part {
								ContentPart::Text(text) => {
									values.push(json!({"type": "text", "text": text}));
								}
								ContentPart::Binary(binary) => {
									let is_image = binary.is_image();
									let Binary {
										content_type, source, ..
									} = binary;

									if is_image {
										match &source {
											BinarySource::Url(url) => {
												values.push(json!({
													"type": "image",
													"source": {
														"type": "url",
														"url": url,
													}
												}));
											}
											BinarySource::Base64(content) => {
												values.push(json!({
													"type": "image",
													"source": {
														"type": "base64",
														"media_type": content_type,
														"data": content,
													}
												}));
											}
										}
									} else {
										match &source {
											BinarySource::Url(url) => {
												values.push(json!({
													"type": "document",
													"source": {
														"type": "url",
														"url": url,
													}
												}));
											}
											BinarySource::Base64(b64) => {
												values.push(json!({
													"type": "document",
													"source": {
														"type": "base64",
														"media_type": content_type,
														"data": b64,
													}
												}));
											}
										}
									}
								}
								// ToolCall is not valid in user content for Anthropic; skip gracefully.
								ContentPart::ToolCall(_tc) => {}
								ContentPart::ToolResponse(tool_response) => {
									values.push(tool_response_to_tool_result(tool_response));
								}
								ContentPart::ThoughtSignature(_) => {}
								ContentPart::ReasoningContent(_) => {}
								ContentPart::Custom(custom_part) => values.push(custom_part.data),
							}
						}
						let values = apply_cache_control_to_parts(cache_control.as_ref(), values);
						messages.push(json!({"role": "user", "content": values}));
					}
				}

				// Assistant can mix text and tool_use entries.
				ChatRole::Assistant => {
					let mut values: Vec<Value> = Vec::new();
					let mut has_tool_use = false;
					let mut has_text = false;

					for part in msg.content {
						match part {
							ContentPart::Text(text) => {
								has_text = true;
								values.push(json!({"type": "text", "text": text}));
							}
							ContentPart::ToolCall(tool_call) => {
								has_tool_use = true;
								// Anthropic API requires `input` to be an object, never null.
								// Streaming parsers may produce null arguments when deltas are
								// missing or empty; fall back to an empty object in that case.
								let input = if tool_call.fn_arguments.is_null() {
									Value::Object(Map::new())
								} else {
									tool_call.fn_arguments
								};
								// see: https://docs.anthropic.com/en/docs/build-with-claude/tool-use#example-of-successful-tool-result
								values.push(json!({
									"type": "tool_use",
									"id": tool_call.call_id,
									"name": tool_call.fn_name,
									"input": input,
								}));
							}
							// Unsupported for assistant role in Anthropic message content
							ContentPart::Binary(_) => {}
							// No provider wire represents a tool result authored by the
							// assistant; fail loudly instead of silently dropping the
							// content (use a Tool-role message).
							ContentPart::ToolResponse(_) => {
								return Err(assistant_embedded_tool_response_err(model_iden));
							}
							ContentPart::ThoughtSignature(_) => {}
							ContentPart::ReasoningContent(_) => {}
							ContentPart::Custom(custom_part) => values.push(custom_part.data),
						}
					}

					if !has_tool_use && has_text && cache_control.is_none() && values.len() == 1 {
						// Optimize to simple string when it's only one text part and no cache control.
						let text = values
							.first()
							.and_then(|v| v.get("text"))
							.and_then(|v| v.as_str())
							.unwrap_or_default()
							.to_string();
						let content = apply_cache_control_to_text(None, text);
						messages.push(json!({"role": "assistant", "content": content}));
					} else {
						let values = apply_cache_control_to_parts(cache_control.as_ref(), values);
						messages.push(json!({"role": "assistant", "content": values}));
					}
				}

				// Tool responses are represented as user tool_result items in Anthropic.
				ChatRole::Tool => {
					let mut values: Vec<Value> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::ToolResponse(tool_response) => {
								values.push(tool_response_to_tool_result(tool_response));
							}
							ContentPart::Custom(custom_part) => values.push(custom_part.data),
							_ => {}
						}
					}
					if !values.is_empty() {
						let values = apply_cache_control_to_parts(cache_control.as_ref(), values);
						messages.push(json!({"role": "user", "content": values}));
					}
				}
			}
		}

		// -- Request-level cache control (Approach B): with no explicit breakpoint, auto-mark the
		// static prefix — the last system block (caches tools+system), else the last tool; no-op otherwise.
		if let Some(req_cc) = request_cache_control {
			let has_tool_cache = chat_req
				.tools
				.as_ref()
				.map(|tools| tools.iter().any(|t| t.cache_control.is_some()))
				.unwrap_or(false);

			if !has_msg_cache && !has_tool_cache {
				if let Some(last_system) = systems.last_mut() {
					last_system.1 = Some(req_cc);
				} else if let Some(last_tool) = chat_req.tools.as_mut().and_then(|tools| tools.last_mut()) {
					last_tool.cache_control = Some(req_cc);
				}
			}
		}

		// -- Create the Anthropic system
		// NOTE: Anthropic does not have a "role": "system", just a single optional system property
		let system = if !systems.is_empty() {
			let has_any_cache = systems.iter().any(|(_, cc)| cc.is_some());
			let system: Value = if has_any_cache {
				// Build multi-part system with per-part cache_control
				let parts: Vec<Value> = systems
					.iter()
					.map(|(content, cc)| {
						if let Some(cc) = cc {
							json!({"type": "text", "text": content, "cache_control": cache_control_to_json(cc)})
						} else {
							json!({"type": "text", "text": content})
						}
					})
					.collect();
				json!(parts)
			} else {
				let content_buff = systems.iter().map(|(content, _)| content.as_str()).collect::<Vec<&str>>();
				// we add empty line in between each system
				let content = content_buff.join("\n\n");
				json!(content)
			};
			Some(system)
		} else {
			None
		};

		// -- Process the tools

		let tools: Option<Vec<Value>> = chat_req
			.tools
			.map(|tools| {
				tools
					.into_iter()
					.map(Self::tool_to_anthropic_tool)
					.collect::<Result<Vec<Value>>>()
			})
			.transpose()?;

		Ok(AnthropicRequestParts {
			system,
			messages,
			tools,
		})
	}

	pub(in crate::adapter::adapters) fn build_web_request_data(
		endpoint: Endpoint,
		auth: AuthData,
		model: ModelIden,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let response_schema = options_set.response_format().and_then(|format| match format {
			ChatResponseFormat::JsonSpec(spec) => Some(sanitize_json_schema(
				&spec.schema,
				JsonSchemaDialect::AnthropicStructured,
			)),
			ChatResponseFormat::JsonMode => None,
		});

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// -- headers
		let headers = Headers::from(vec![
			("x-api-key".to_string(), api_key),
			("anthropic-version".to_string(), ANTHROPIC_VERSION.to_string()),
		]);

		// -- Parts
		let AnthropicRequestParts {
			system,
			messages,
			tools,
		} = Self::into_anthropic_request_parts(&model, chat_req, options_set.cache_control().cloned())?;

		// -- Extract Model Name and Reasoning
		let (_, raw_model_name) = model.model_name.namespace_and_name();

		// -- Reasoning Budget
		let (model_name, computed_reasoning_effort) = match (raw_model_name, options_set.reasoning_effort()) {
			// No explicity reasoning_effor, try to infer from model name suffix (supports -zero and -none)
			(model, None) => {
				// let model_name: &str = &model.model_name;
				let (reasoning, model_name) = ReasoningEffort::from_model_name(model);
				// 'zero' is canonical; 'none' is backward-compat alias
				// create the model name if there was a `-..` reasoning suffix
				(model_name, reasoning)
			}
			// If reasoning effort, turn the low, medium, budget ones into Budget
			(model, Some(effort)) => (model, Some(effort.clone())),
		};
		let anthropic_model = AnthropicModel::parse(model_name);
		let capabilities = anthropic_model.capabilities();

		// -- Build the basic payload
		let stream = matches!(service_type, ServiceType::ChatStream);
		let mut payload = json!({
			"model": anthropic_model.normalized_name.to_string(),
			"stream": stream
		});

		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}

		// -- Tools (before messages)
		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}
		if let Some(tool_choice) = anthropic_tool_choice(options_set.tool_choice()) {
			payload.x_insert("tool_choice", tool_choice)?;
		}

		// -- Messages (after tools)
		payload.x_insert("messages", messages)?;

		// -- Set the reasoning effort
		// Both reasoning effort and structured-output format write into `output_config`.
		// Build a shared map so both contributions end up in the same object.
		let mut output_config: Map<String, Value> = Map::new();

		if let Some(computed_reasoning_effort) = computed_reasoning_effort {
			let capture_reasoning_content = options_set.capture_reasoning_content().unwrap_or_default();
			insert_anthropic_reasoning(
				&mut payload,
				&mut output_config,
				&capabilities,
				&computed_reasoning_effort,
				capture_reasoning_content,
			)?;
		}

		// -- Add supported ChatOptions
		if let Some(schema) = response_schema {
			// https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-outputs
			output_config.insert(
				"format".to_string(),
				json!({
					"type": "json_schema",
					"schema": schema,
				}),
			);
		}

		// Insert output_config once, merging effort + format into a single object.
		if !output_config.is_empty() {
			payload.x_insert("output_config", Value::Object(output_config))?;
		}

		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop_sequences", options_set.stop_sequences())?;
		}

		let max_tokens = Self::resolve_max_tokens_for_capabilities(&capabilities, &options_set);
		payload.x_insert("max_tokens", max_tokens)?; // required for Anthropic

		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		if let Some(extra_body) = options_set.extra_body() {
			payload.x_merge(extra_body.clone())?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	pub(in crate::adapter::adapters) fn build_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the provider_model_iden
		// TODO: Need to be implemented (if available), for now, just clone model_iden
		let provider_model_name: Option<String> = body.x_remove("model").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);

		// -- Capture the usage
		let usage = body.x_take::<Value>("usage");

		let usage = usage.map(Self::into_usage).unwrap_or_default();
		let stop_reason = body
			.x_take::<Option<String>>("stop_reason")
			.ok()
			.flatten()
			.map(StopReason::from);

		// -- Capture the content
		let mut content: MessageContent = MessageContent::default();

		// NOTE: Here we are going to concatenate all of the Anthropic text content items into one
		//       genai MessageContent::Text. This is more in line with the OpenAI API style,
		//       but loses the fact that they were originally separate items.
		let json_content_items: Vec<Value> = body.x_take("content")?;

		let mut reasoning_content: Vec<String> = Vec::new();

		for mut item in json_content_items {
			let typ: String = item.x_take("type")?;
			match typ.as_ref() {
				"text" => {
					let part = ContentPart::from_text(item.x_take::<String>("text")?);
					content.push(part);
				}
				"thinking" => reasoning_content.push(item.x_take("thinking")?),
				"tool_use" => {
					let call_id = item.x_take::<String>("id")?;
					let fn_name = item.x_take::<String>("name")?;
					// if not found, will be Value::Null
					let fn_arguments = item.x_take::<Value>("input").unwrap_or_default();
					let tool_call = ToolCall {
						call_id,
						fn_name,
						fn_arguments,
						thought_signatures: None,
					};

					let part = ContentPart::ToolCall(tool_call);
					content.push(part);
				}
				other_typ => {
					// insert it back
					item.x_insert("type", other_typ)?;
					content.push(ContentPart::from_custom(item, Some(model_iden.clone())))
				}
			}
		}

		let reasoning_content = if !reasoning_content.is_empty() {
			Some(reasoning_content.join("\n"))
		} else {
			None
		};

		Ok(ChatResponse {
			content,
			reasoning_content,
			model_iden,
			provider_model_iden,
			stop_reason,
			usage,
			captured_raw_body: None, // Set by the client exec_chat
			response_id: None,
		})
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
		let api_key = auth.single_key_value().ok();
		let headers = api_key
			.map(|api_key| {
				Headers::from(vec![
					("x-api-key".to_string(), api_key),
					("anthropic-version".to_string(), ANTHROPIC_VERSION.to_string()),
				])
			})
			.unwrap_or_default();

		// -- Exec request
		let mut res = web_client
			.do_get(&url, &headers)
			.await
			.map_err(|webc_error| crate::Error::WebAdapterCall {
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
		}

		Ok(models)
	}

	fn tool_to_anthropic_tool(tool: Tool) -> Result<Value> {
		let Tool {
			name,
			description,
			schema,
			config,
			cache_control,
			eager_input_streaming,
			strict,
			..
		} = tool;

		let name = match name {
			ToolName::WebSearch => "web_search".to_string(),
			ToolName::Custom(name) => name,
		};

		let mut tool_value = json!({"name": name});

		// -- Add type for builtin tool
		#[allow(clippy::single_match)] // will have more
		match name.as_str() {
			"web_search" => {
				tool_value.x_insert("type", "web_search_20250305")?;
			}
			_ => (),
		}

		// NOTE: Fo now, if tool_value.type then, assume bultin and set config as propertie
		if tool_value.get("type").is_some() {
			if let Some(config) = config {
				match config {
					ToolConfig::WebSearch(config) => {
						if let Some(max_uses) = config.max_uses {
							let _ = tool_value.x_insert("max_uses", max_uses);
						}
						if let Some(allowed_domains) = config.allowed_domains {
							let _ = tool_value.x_insert("allowed_domains", allowed_domains);
						}
						if let Some(blocked_domains) = config.blocked_domains {
							let _ = tool_value.x_insert("blocked_domains", blocked_domains);
						}
					}
					// if custom, we assume we flatten the config properties since we are in a builtin
					ToolConfig::Custom(config) => {
						// NOTE: For now, ignore if not object
						tool_value.x_merge(config)?;
					}
				}
			}
		} else {
			let schema = if strict == Some(true) {
				schema.map(|schema| sanitize_json_schema(&schema, JsonSchemaDialect::AnthropicStructured))
			} else {
				schema
			};
			tool_value.x_insert("input_schema", schema)?;
			if let Some(strict) = strict {
				tool_value.x_insert("strict", strict)?;
			}
			if let Some(description) = description {
				// TODO: need to handle error
				let _ = tool_value.x_insert("description", description);
			}
			// Anthropic fine-grained tool streaming (GA): opt-in per tool.
			if eager_input_streaming == Some(true) {
				let _ = tool_value.x_insert("eager_input_streaming", true);
			}
		}

		if let Some(cc) = cache_control {
			let _ = tool_value.x_insert("cache_control", cache_control_to_json(&cc));
		}

		Ok(tool_value)
	}
}

pub(in crate::adapter) struct AnthropicRequestParts {
	pub system: Option<Value>,
	pub messages: Vec<Value>,
	pub tools: Option<Vec<Value>>,
}

// region:    --- Support

/// Convert CacheControl to Anthropic JSON format.
///
/// See: https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching#1-hour-cache-duration
fn cache_control_to_json(cache_control: &CacheControl) -> Value {
	match cache_control {
		CacheControl::Ephemeral => {
			json!({"type": "ephemeral"})
		}
		CacheControl::Memory => {
			json!({"type": "ephemeral"})
		}
		CacheControl::Ephemeral5m => {
			json!({"type": "ephemeral", "ttl": "5m"})
		}
		CacheControl::Ephemeral1h => {
			json!({"type": "ephemeral", "ttl": "1h"})
		}
		// Anthropic's max ephemeral TTL is 1h, so 24h clamps to 1h (see CacheControl::Ephemeral24h docs).
		CacheControl::Ephemeral24h => {
			json!({"type": "ephemeral", "ttl": "1h"})
		}
	}
}

/// Parse cache_creation breakdown from Anthropic API response.
///
/// The API returns TTL-specific token counts in the `cache_creation` object:
/// ```json
/// "cache_creation": {
///     "ephemeral_5m_input_tokens": 456,
///     "ephemeral_1h_input_tokens": 100
/// }
/// ```
pub(super) fn parse_cache_creation_details(cache_creation: &Value) -> Option<CacheCreationDetails> {
	let ephemeral_5m_tokens = cache_creation
		.get("ephemeral_5m_input_tokens")
		.and_then(|v| v.as_i64())
		.map(|v| v as i32);
	let ephemeral_1h_tokens = cache_creation
		.get("ephemeral_1h_input_tokens")
		.and_then(|v| v.as_i64())
		.map(|v| v as i32);

	// Only return Some if at least one TTL has tokens
	if ephemeral_5m_tokens.is_some() || ephemeral_1h_tokens.is_some() {
		Some(CacheCreationDetails {
			ephemeral_5m_tokens,
			ephemeral_1h_tokens,
		})
	} else {
		None
	}
}

/// Apply the cache control logic to a text content
fn apply_cache_control_to_text(cache_control: Option<&CacheControl>, content: String) -> Value {
	if let Some(cc) = cache_control {
		let value = json!({"type": "text", "text": content, "cache_control": cache_control_to_json(cc)});
		json!(vec![value])
	}
	// simple return
	else {
		json!(content)
	}
}

/// Apply the cache control logic to a text content
fn apply_cache_control_to_parts(cache_control: Option<&CacheControl>, parts: Vec<Value>) -> Vec<Value> {
	let mut parts = parts;
	if let Some(cc) = cache_control
		&& !parts.is_empty()
	{
		let len = parts.len();
		if let Some(last_value) = parts.get_mut(len - 1) {
			// NOTE: For now, if it fails, then, no cache
			let _ = last_value.x_insert("cache_control", cache_control_to_json(cc));
			// TODO: Should warn
		}
	}
	parts
}

/// Serialize a `ToolResponse` into an Anthropic `tool_result` content item.
///
/// - Without binary parts, `content` remains a plain string (legacy shape, unchanged).
/// - With parts, `content` becomes an array of a `text` block (when the text is non-empty)
///   followed by `image` blocks for image parts (native `base64` or `url` source).
///
/// NOTE: Anthropic `tool_result` content only accepts `text` and `image` blocks,
///       so non-image parts are skipped with a warning. Image sources serialize
///       the same way as in user-message image handling above.
fn tool_response_to_tool_result(tool_response: ToolResponse) -> Value {
	let ToolResponse {
		call_id,
		content,
		parts,
		..
	} = tool_response;

	let parts = parts.unwrap_or_default();

	if parts.is_empty() {
		return json!({
			"type": "tool_result",
			"content": content,
			"tool_use_id": call_id,
		});
	}

	let mut values: Vec<Value> = Vec::new();
	if !content.is_empty() {
		values.push(json!({"type": "text", "text": content}));
	}

	for binary in parts {
		if !binary.is_image() {
			warn!(
				"Anthropic tool_result only supports text and image blocks; skipping non-image part '{}'",
				binary.content_type
			);
			continue;
		}
		let Binary {
			content_type, source, ..
		} = binary;
		match source {
			BinarySource::Base64(data) => {
				values.push(json!({
					"type": "image",
					"source": {
						"type": "base64",
						"media_type": content_type,
						"data": data,
					}
				}));
			}
			BinarySource::Url(url) => {
				values.push(json!({
					"type": "image",
					"source": {
						"type": "url",
						"url": url,
					}
				}));
			}
		}
	}

	// If all parts were skipped and there is no text, fall back to the legacy string shape.
	if values.is_empty() {
		return json!({
			"type": "tool_result",
			"content": content,
			"tool_use_id": call_id,
		});
	}

	json!({
		"type": "tool_result",
		"content": values,
		"tool_use_id": call_id,
	})
}

fn anthropic_tool_choice(tool_choice: Option<&ToolChoice>) -> Option<Value> {
	match tool_choice? {
		ToolChoice::Auto => Some(json!({"type": "auto"})),
		ToolChoice::None => Some(json!({"type": "none"})),
		ToolChoice::Required => Some(json!({"type": "any"})),
		ToolChoice::Tool { name } => Some(json!({
			"type": "tool",
			"name": name
		})),
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "adapter_shared_tests.rs"]
mod tests;

// endregion: --- Tests
