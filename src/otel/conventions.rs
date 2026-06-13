//! OpenTelemetry GenAI semantic-convention constants and value mappings.
//!
//! Attribute names, operation names, and provider names follow the
//! [OpenTelemetry GenAI semantic conventions](https://github.com/open-telemetry/semantic-conventions-genai)
//! (status: Development). They are centralized here so that any future churn in
//! the spec is absorbed in a single place. A snapshot of the spec lives under
//! `docs/otel-genai-spec/`.

use crate::adapter::AdapterKind;

// region:    --- Span / "otel.*" bridge fields

// These are the magic fields `tracing-opentelemetry` reads to shape the OTel span.
/// Overrides the OTel span name (since `tracing` span names must be static).
pub const OTEL_NAME: &str = "otel.name";
/// OTel span kind (`client`, `server`, `internal`, ...).
pub const OTEL_KIND: &str = "otel.kind";
/// OTel span status code (`ok` / `error`).
pub const OTEL_STATUS_CODE: &str = "otel.status_code";

/// `otel.kind` value for client spans.
pub const KIND_CLIENT: &str = "client";
/// `otel.kind` value for internal spans.
pub const KIND_INTERNAL: &str = "internal";
/// `otel.status_code` value for errored operations.
pub const STATUS_ERROR: &str = "error";

// endregion: --- Span / "otel.*" bridge fields

// region:    --- gen_ai.* attribute keys

pub const OPERATION_NAME: &str = "gen_ai.operation.name";
pub const PROVIDER_NAME: &str = "gen_ai.provider.name";
pub const ERROR_TYPE: &str = "error.type";
pub const CONVERSATION_ID: &str = "gen_ai.conversation.id";
pub const OUTPUT_TYPE: &str = "gen_ai.output.type";

pub const REQUEST_MODEL: &str = "gen_ai.request.model";
pub const REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";
pub const REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";
pub const REQUEST_TOP_P: &str = "gen_ai.request.top_p";
pub const REQUEST_TOP_K: &str = "gen_ai.request.top_k";
pub const REQUEST_STOP_SEQUENCES: &str = "gen_ai.request.stop_sequences";
pub const REQUEST_SEED: &str = "gen_ai.request.seed";
pub const REQUEST_CHOICE_COUNT: &str = "gen_ai.request.choice.count";
pub const REQUEST_STREAM: &str = "gen_ai.request.stream";
pub const REQUEST_FREQUENCY_PENALTY: &str = "gen_ai.request.frequency_penalty";
pub const REQUEST_PRESENCE_PENALTY: &str = "gen_ai.request.presence_penalty";
pub const REQUEST_ENCODING_FORMATS: &str = "gen_ai.request.encoding_formats";
pub const EMBEDDINGS_DIMENSION_COUNT: &str = "gen_ai.embeddings.dimension.count";

pub const RESPONSE_ID: &str = "gen_ai.response.id";
pub const RESPONSE_MODEL: &str = "gen_ai.response.model";
pub const RESPONSE_FINISH_REASONS: &str = "gen_ai.response.finish_reasons";
pub const RESPONSE_TIME_TO_FIRST_CHUNK: &str = "gen_ai.response.time_to_first_chunk";

pub const USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";
pub const USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
pub const USAGE_CACHE_READ_INPUT_TOKENS: &str = "gen_ai.usage.cache_read.input_tokens";
pub const USAGE_CACHE_CREATION_INPUT_TOKENS: &str = "gen_ai.usage.cache_creation.input_tokens";
pub const USAGE_REASONING_OUTPUT_TOKENS: &str = "gen_ai.usage.reasoning.output_tokens";

pub const SERVER_ADDRESS: &str = "server.address";
pub const SERVER_PORT: &str = "server.port";

// -- Opt-in content (sensitive; see `attrs::capture_content`)
pub const SYSTEM_INSTRUCTIONS: &str = "gen_ai.system_instructions";
pub const INPUT_MESSAGES: &str = "gen_ai.input.messages";
pub const OUTPUT_MESSAGES: &str = "gen_ai.output.messages";
pub const TOOL_DEFINITIONS: &str = "gen_ai.tool.definitions";

// -- Agent / workflow / tool spans (opt-in helpers)
pub const AGENT_NAME: &str = "gen_ai.agent.name";
pub const AGENT_ID: &str = "gen_ai.agent.id";
pub const AGENT_DESCRIPTION: &str = "gen_ai.agent.description";
pub const WORKFLOW_NAME: &str = "gen_ai.workflow.name";
pub const TOOL_NAME: &str = "gen_ai.tool.name";
pub const TOOL_CALL_ID: &str = "gen_ai.tool.call.id";
pub const TOOL_DESCRIPTION: &str = "gen_ai.tool.description";
pub const TOOL_TYPE: &str = "gen_ai.tool.type";
pub const TOOL_CALL_ARGUMENTS: &str = "gen_ai.tool.call.arguments";
pub const TOOL_CALL_RESULT: &str = "gen_ai.tool.call.result";

// -- Evaluation event
pub const EVALUATION_NAME: &str = "gen_ai.evaluation.name";
pub const EVALUATION_SCORE_VALUE: &str = "gen_ai.evaluation.score.value";
pub const EVALUATION_SCORE_LABEL: &str = "gen_ai.evaluation.score.label";
pub const EVALUATION_EXPLANATION: &str = "gen_ai.evaluation.explanation";

// endregion: --- gen_ai.* attribute keys

// region:    --- gen_ai.operation.name values

pub const OP_CHAT: &str = "chat";
pub const OP_EMBEDDINGS: &str = "embeddings";
pub const OP_EXECUTE_TOOL: &str = "execute_tool";
pub const OP_CREATE_AGENT: &str = "create_agent";
pub const OP_INVOKE_AGENT: &str = "invoke_agent";
pub const OP_INVOKE_WORKFLOW: &str = "invoke_workflow";
pub const OP_PLAN: &str = "plan";

// endregion: --- gen_ai.operation.name values

// region:    --- gen_ai.output.type values

pub const OUTPUT_TYPE_TEXT: &str = "text";
pub const OUTPUT_TYPE_JSON: &str = "json";

// endregion: --- gen_ai.output.type values

// region:    --- Provider name mapping

/// Maps a genai [`AdapterKind`] to the `gen_ai.provider.name` value.
///
/// Uses the spec's well-known provider names where one exists; otherwise falls
/// back to the adapter's lower-case identifier (the spec permits custom values).
pub fn provider_name(adapter_kind: AdapterKind) -> &'static str {
	match adapter_kind {
		AdapterKind::OpenAI | AdapterKind::OpenAIResp => "openai",
		AdapterKind::Anthropic => "anthropic",
		AdapterKind::Gemini => "gcp.gemini",
		AdapterKind::Vertex => "gcp.vertex_ai",
		AdapterKind::BedrockApi => "aws.bedrock",
		#[cfg(feature = "bedrock-sigv4")]
		AdapterKind::BedrockSigv4 => "aws.bedrock",
		AdapterKind::Groq => "groq",
		AdapterKind::Cohere => "cohere",
		AdapterKind::DeepSeek => "deepseek",
		AdapterKind::Xai => "x_ai",
		AdapterKind::Moonshot => "moonshot_ai",
		// -- Not in the spec's well-known list: fall back to the adapter's lower-case id.
		other => other.as_lower_str(),
	}
}

// endregion: --- Provider name mapping

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_otel_provider_name_well_known() {
		assert_eq!(provider_name(AdapterKind::OpenAI), "openai");
		assert_eq!(provider_name(AdapterKind::OpenAIResp), "openai");
		assert_eq!(provider_name(AdapterKind::Anthropic), "anthropic");
		assert_eq!(provider_name(AdapterKind::Gemini), "gcp.gemini");
		assert_eq!(provider_name(AdapterKind::Vertex), "gcp.vertex_ai");
		assert_eq!(provider_name(AdapterKind::BedrockApi), "aws.bedrock");
		assert_eq!(provider_name(AdapterKind::Groq), "groq");
		assert_eq!(provider_name(AdapterKind::Cohere), "cohere");
		assert_eq!(provider_name(AdapterKind::DeepSeek), "deepseek");
		assert_eq!(provider_name(AdapterKind::Xai), "x_ai");
		assert_eq!(provider_name(AdapterKind::Moonshot), "moonshot_ai");
	}

	#[test]
	fn test_otel_provider_name_fallback() {
		// Not in the spec's well-known list → adapter lower-case id.
		assert_eq!(provider_name(AdapterKind::Ollama), AdapterKind::Ollama.as_lower_str());
		assert_eq!(
			provider_name(AdapterKind::Together),
			AdapterKind::Together.as_lower_str()
		);
		assert_eq!(
			provider_name(AdapterKind::Fireworks),
			AdapterKind::Fireworks.as_lower_str()
		);
	}
}
