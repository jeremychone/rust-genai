//! Builders and recorders for the auto-instrumented GenAI client spans
//! (`chat` inference and `embeddings`), following the OTel GenAI spec.
//!
//! The spans are plain `tracing` spans whose field names follow the
//! `gen_ai.*` conventions; an application that wires `tracing-opentelemetry`
//! onto its subscriber turns them into spec-compliant OpenTelemetry spans.
//! `otel.name` / `otel.kind` / `otel.status_code` are the bridge fields the
//! `tracing-opentelemetry` layer interprets.
//!
//! All optional attributes are declared with [`tracing::field::Empty`] at span
//! creation and recorded later only when a value is available, so unset
//! attributes do not appear on the exported span.

use crate::ModelIden;
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, StreamEnd, Usage};
use crate::embed::{EmbedOptionsSet, EmbedResponse};
use crate::otel::{attrs, conventions as c, error as otel_error};
use crate::resolver::Endpoint;
use crate::{Error, Result};
use tracing::Span;
use tracing::field::Empty;

// region:    --- Chat (inference) span

/// Creates the `chat` inference span and records the request attributes.
///
/// Returns the [`Span`]; the caller keeps it alive for the duration of the
/// operation and feeds the response/usage/error into the `record_*` helpers.
pub(crate) fn chat_request_span(
	model_iden: &ModelIden,
	endpoint: &Endpoint,
	options: &ChatOptionsSet,
	chat_req: &ChatRequest,
	stream: bool,
) -> Span {
	let provider = c::provider_name(model_iden.adapter_kind);
	let model = model_iden.model_name.as_str();
	let span_name = format!("{} {model}", c::OP_CHAT);

	let span = tracing::info_span!(
		"genai.chat",
		"otel.name" = span_name.as_str(),
		"otel.kind" = c::KIND_CLIENT,
		"otel.status_code" = Empty,
		"error.type" = Empty,
		"gen_ai.operation.name" = c::OP_CHAT,
		"gen_ai.provider.name" = provider,
		"gen_ai.request.model" = model,
		"gen_ai.request.stream" = stream,
		"gen_ai.request.temperature" = Empty,
		"gen_ai.request.max_tokens" = Empty,
		"gen_ai.request.top_p" = Empty,
		"gen_ai.request.seed" = Empty,
		"gen_ai.request.stop_sequences" = Empty,
		"gen_ai.output.type" = Empty,
		"server.address" = Empty,
		"server.port" = Empty,
		"gen_ai.response.id" = Empty,
		"gen_ai.response.model" = Empty,
		"gen_ai.response.finish_reasons" = Empty,
		"gen_ai.usage.input_tokens" = Empty,
		"gen_ai.usage.output_tokens" = Empty,
		"gen_ai.usage.cache_read.input_tokens" = Empty,
		"gen_ai.usage.cache_creation.input_tokens" = Empty,
		"gen_ai.usage.reasoning.output_tokens" = Empty,
		"gen_ai.response.time_to_first_chunk" = Empty,
		"gen_ai.system_instructions" = Empty,
		"gen_ai.input.messages" = Empty,
		"gen_ai.output.messages" = Empty,
		"gen_ai.tool.definitions" = Empty,
	);

	record_server(&span, endpoint);
	record_chat_request_options(&span, options);

	// -- Opt-in (sensitive) request content.
	if attrs::capture_content() {
		if let Some(instructions) = attrs::system_instructions_json(chat_req.iter_systems()) {
			span.record(c::SYSTEM_INSTRUCTIONS, instructions.as_str());
		}
		span.record(
			c::INPUT_MESSAGES,
			attrs::input_messages_json(&chat_req.messages).as_str(),
		);
		if let Some(tools) = chat_req.tools.as_ref() {
			span.record(c::TOOL_DEFINITIONS, attrs::tool_definitions_json(tools).as_str());
		}
	}

	span
}

fn record_chat_request_options(span: &Span, options: &ChatOptionsSet) {
	if let Some(temperature) = options.temperature() {
		span.record(c::REQUEST_TEMPERATURE, temperature);
	}
	if let Some(max_tokens) = options.max_tokens() {
		span.record(c::REQUEST_MAX_TOKENS, i64::from(max_tokens));
	}
	if let Some(top_p) = options.top_p() {
		span.record(c::REQUEST_TOP_P, top_p);
	}
	if let Some(seed) = options.seed() {
		span.record(c::REQUEST_SEED, seed as i64);
	}
	let stop_sequences = options.stop_sequences();
	if !stop_sequences.is_empty()
		&& let Ok(json) = serde_json::to_string(stop_sequences)
	{
		span.record(c::REQUEST_STOP_SEQUENCES, json.as_str());
	}
	// `gen_ai.output.type` is only set when an output format is requested.
	if options.response_format().is_some() {
		span.record(c::OUTPUT_TYPE, c::OUTPUT_TYPE_JSON);
	}
}

/// Records the response attributes (id, model, finish reasons, usage, and —
/// when content capture is enabled — the output messages).
pub(crate) fn record_chat_response(span: &Span, res: &ChatResponse) {
	if let Some(response_id) = res.response_id.as_deref() {
		span.record(c::RESPONSE_ID, response_id);
	}
	span.record(c::RESPONSE_MODEL, res.provider_model_iden.model_name.as_str());
	if let Some(stop_reason) = res.stop_reason.as_ref() {
		span.record(
			c::RESPONSE_FINISH_REASONS,
			attrs::finish_reasons_json(stop_reason).as_str(),
		);
	}
	record_usage(span, &res.usage);

	if attrs::capture_content() {
		let messages = attrs::output_messages_json(&res.content, res.stop_reason.as_ref());
		span.record(c::OUTPUT_MESSAGES, messages.as_str());
	}
}

/// Records token usage on the span. Counts are emitted only when present.
pub(crate) fn record_usage(span: &Span, usage: &Usage) {
	if let Some(input_tokens) = usage.prompt_tokens {
		span.record(c::USAGE_INPUT_TOKENS, i64::from(input_tokens));
	}
	if let Some(output_tokens) = usage.completion_tokens {
		span.record(c::USAGE_OUTPUT_TOKENS, i64::from(output_tokens));
	}
	if let Some(details) = usage.prompt_tokens_details.as_ref() {
		if let Some(cached) = details.cached_tokens {
			span.record(c::USAGE_CACHE_READ_INPUT_TOKENS, i64::from(cached));
		}
		if let Some(cache_creation) = details.cache_creation_tokens {
			span.record(c::USAGE_CACHE_CREATION_INPUT_TOKENS, i64::from(cache_creation));
		}
	}
	if let Some(details) = usage.completion_tokens_details.as_ref()
		&& let Some(reasoning) = details.reasoning_tokens
	{
		span.record(c::USAGE_REASONING_OUTPUT_TOKENS, i64::from(reasoning));
	}
}

// endregion: --- Chat (inference) span

// region:    --- Streaming helpers

/// Records `gen_ai.response.time_to_first_chunk` (in seconds) on a streaming span.
pub(crate) fn record_time_to_first_chunk(span: &Span, seconds: f64) {
	span.record(c::RESPONSE_TIME_TO_FIRST_CHUNK, seconds);
}

/// Records the captured response attributes from a streaming `End` event:
/// usage, finish reason, response id, and — when content capture is enabled —
/// the output messages.
pub(crate) fn record_stream_end(span: &Span, end: &StreamEnd) {
	if let Some(response_id) = end.captured_response_id.as_deref() {
		span.record(c::RESPONSE_ID, response_id);
	}
	if let Some(stop_reason) = end.captured_stop_reason.as_ref() {
		span.record(
			c::RESPONSE_FINISH_REASONS,
			attrs::finish_reasons_json(stop_reason).as_str(),
		);
	}
	if let Some(usage) = end.captured_usage.as_ref() {
		record_usage(span, usage);
	}
	if attrs::capture_content()
		&& let Some(content) = end.captured_content.as_ref()
	{
		let messages = attrs::output_messages_json(content, end.captured_stop_reason.as_ref());
		span.record(c::OUTPUT_MESSAGES, messages.as_str());
	}
}

// endregion: --- Streaming helpers

// region:    --- Embeddings span

/// Creates the `embeddings` span and records the request attributes.
pub(crate) fn embeddings_request_span(model_iden: &ModelIden, endpoint: &Endpoint, options: &EmbedOptionsSet) -> Span {
	let provider = c::provider_name(model_iden.adapter_kind);
	let model = model_iden.model_name.as_str();
	let span_name = format!("{} {model}", c::OP_EMBEDDINGS);

	let span = tracing::info_span!(
		"genai.embeddings",
		"otel.name" = span_name.as_str(),
		"otel.kind" = c::KIND_CLIENT,
		"otel.status_code" = Empty,
		"error.type" = Empty,
		"gen_ai.operation.name" = c::OP_EMBEDDINGS,
		"gen_ai.provider.name" = provider,
		"gen_ai.request.model" = model,
		"gen_ai.embeddings.dimension.count" = Empty,
		"gen_ai.request.encoding_formats" = Empty,
		"server.address" = Empty,
		"server.port" = Empty,
		"gen_ai.response.model" = Empty,
		"gen_ai.usage.input_tokens" = Empty,
	);

	record_server(&span, endpoint);
	if let Some(dimensions) = options.dimensions() {
		span.record(c::EMBEDDINGS_DIMENSION_COUNT, dimensions as i64);
	}
	if let Some(encoding_format) = options.encoding_format()
		&& let Ok(json) = serde_json::to_string(&[encoding_format])
	{
		span.record(c::REQUEST_ENCODING_FORMATS, json.as_str());
	}

	span
}

/// Records the embeddings response attributes (response model + input tokens).
pub(crate) fn record_embed_response(span: &Span, res: &EmbedResponse) {
	span.record(c::RESPONSE_MODEL, res.provider_model_iden.model_name.as_str());
	if let Some(input_tokens) = res.usage.prompt_tokens {
		span.record(c::USAGE_INPUT_TOKENS, i64::from(input_tokens));
	}
}

// endregion: --- Embeddings span

// region:    --- Shared recorders

/// Records `server.address` / `server.port` parsed from the endpoint base URL.
fn record_server(span: &Span, endpoint: &Endpoint) {
	let (address, port) = attrs::server_address_and_port(endpoint.base_url());
	if let Some(address) = address {
		span.record(c::SERVER_ADDRESS, address.as_str());
	}
	if let Some(port) = port {
		span.record(c::SERVER_PORT, i64::from(port));
	}
}

/// Records the error status (`otel.status_code = error`) and `error.type` for a
/// failed operation.
pub(crate) fn record_error(span: &Span, error: &Error) {
	span.record(c::OTEL_STATUS_CODE, c::STATUS_ERROR);
	span.record(c::ERROR_TYPE, otel_error::error_type(error).as_str());
}

/// Records the outcome of an operation onto its span: response attributes on
/// success, error status on failure. Returns the result unchanged.
pub(crate) fn record_chat_result(span: &Span, result: Result<ChatResponse>) -> Result<ChatResponse> {
	match &result {
		Ok(res) => record_chat_response(span, res),
		Err(err) => record_error(span, err),
	}
	result
}

/// Records the outcome of an embeddings operation onto its span.
pub(crate) fn record_embed_result(span: &Span, result: Result<EmbedResponse>) -> Result<EmbedResponse> {
	match &result {
		Ok(res) => record_embed_response(span, res),
		Err(err) => record_error(span, err),
	}
	result
}

// endregion: --- Shared recorders
