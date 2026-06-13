//! Integration tests for span/event field emission.
//!
//! These install a capturing [`tracing_subscriber::Layer`] for the duration of a
//! closure and assert on the exact `gen_ai.*` fields genai records. This
//! validates what genai owns (the field names/values); the field → OpenTelemetry
//! attribute mapping is `tracing-opentelemetry`'s own (separately tested)
//! responsibility.

use crate::ModelIden;
use crate::adapter::AdapterKind;
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{
	ChatMessage, ChatOptions, ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, MessageContent, StopReason, Usage,
};
use crate::otel::{agent, events, span};
use crate::resolver::Endpoint;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

// region:    --- Capture layer

/// A captured span: its (static) metadata name and recorded field values
/// (formatted as strings). Empty/unrecorded fields are absent from `fields`.
#[derive(Clone, Debug, Default)]
struct CapturedSpan {
	name: String,
	fields: HashMap<String, String>,
}

impl CapturedSpan {
	fn get(&self, key: &str) -> Option<&str> {
		self.fields.get(key).map(|s| s.as_str())
	}

	fn has(&self, key: &str) -> bool {
		self.fields.contains_key(key)
	}
}

#[derive(Clone, Default)]
struct Captured {
	spans: Arc<Mutex<Vec<(u64, CapturedSpan)>>>,
}

impl Captured {
	/// Returns the first captured span whose metadata name matches.
	fn by_name(&self, name: &str) -> CapturedSpan {
		self.spans
			.lock()
			.unwrap()
			.iter()
			.find(|(_, span)| span.name == name)
			.map(|(_, span)| span.clone())
			.unwrap_or_else(|| panic!("no captured span named `{name}`"))
	}
}

struct FieldVisitor<'a>(&'a mut HashMap<String, String>);

impl Visit for FieldVisitor<'_> {
	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		self.0.insert(field.name().to_string(), format!("{value:?}"));
	}
	fn record_str(&mut self, field: &Field, value: &str) {
		self.0.insert(field.name().to_string(), value.to_string());
	}
	fn record_i64(&mut self, field: &Field, value: i64) {
		self.0.insert(field.name().to_string(), value.to_string());
	}
	fn record_u64(&mut self, field: &Field, value: u64) {
		self.0.insert(field.name().to_string(), value.to_string());
	}
	fn record_f64(&mut self, field: &Field, value: f64) {
		self.0.insert(field.name().to_string(), value.to_string());
	}
	fn record_bool(&mut self, field: &Field, value: bool) {
		self.0.insert(field.name().to_string(), value.to_string());
	}
}

struct CaptureLayer {
	captured: Captured,
}

impl<S> Layer<S> for CaptureLayer
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, _ctx: Context<'_, S>) {
		let mut span = CapturedSpan {
			name: attrs.metadata().name().to_string(),
			fields: HashMap::new(),
		};
		attrs.record(&mut FieldVisitor(&mut span.fields));
		self.captured.spans.lock().unwrap().push((id.into_u64(), span));
	}

	fn on_record(&self, id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
		let mut spans = self.captured.spans.lock().unwrap();
		if let Some((_, span)) = spans.iter_mut().find(|(span_id, _)| *span_id == id.into_u64()) {
			values.record(&mut FieldVisitor(&mut span.fields));
		}
	}
}

/// Runs `f` with a capturing subscriber installed on the current thread and
/// returns the captured spans.
fn capture(f: impl FnOnce()) -> Captured {
	let captured = Captured::default();
	let layer = CaptureLayer {
		captured: captured.clone(),
	};
	let subscriber = tracing_subscriber::registry().with(layer);
	tracing::subscriber::with_default(subscriber, f);
	captured
}

// endregion: --- Capture layer

// region:    --- Fixtures

fn model() -> ModelIden {
	ModelIden::from_static(AdapterKind::OpenAI, "gpt-4o")
}

fn endpoint() -> Endpoint {
	Endpoint::from_static("https://api.openai.com/v1/")
}

fn chat_response() -> ChatResponse {
	ChatResponse {
		content: MessageContent::from_text("Hello!"),
		reasoning_content: None,
		model_iden: model(),
		provider_model_iden: ModelIden::from_static(AdapterKind::OpenAI, "gpt-4o-2024-08-06"),
		stop_reason: Some(StopReason::Completed("stop".to_string())),
		usage: Usage {
			prompt_tokens: Some(10),
			completion_tokens: Some(20),
			total_tokens: Some(30),
			..Default::default()
		},
		captured_raw_body: None,
		response_id: Some("chatcmpl-123".to_string()),
	}
}

// endregion: --- Fixtures

// region:    --- Chat span tests

#[test]
fn test_otel_chat_span_request_and_response_attributes() {
	let captured = capture(|| {
		let options = ChatOptions::default().with_temperature(0.7).with_max_tokens(256);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));
		let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello")]);

		let span = span::chat_request_span(&model(), &endpoint(), &options_set, &chat_req, false);
		span::record_chat_response(&span, &chat_response());
	});

	let span = captured.by_name("genai.chat");

	// -- Span shape / required attributes
	assert_eq!(span.get("otel.name"), Some("chat gpt-4o"));
	assert_eq!(span.get("otel.kind"), Some("client"));
	assert_eq!(span.get("gen_ai.operation.name"), Some("chat"));
	assert_eq!(span.get("gen_ai.provider.name"), Some("openai"));
	assert_eq!(span.get("gen_ai.request.model"), Some("gpt-4o"));
	assert_eq!(span.get("gen_ai.request.stream"), Some("false"));

	// -- Request options
	assert_eq!(span.get("gen_ai.request.temperature"), Some("0.7"));
	assert_eq!(span.get("gen_ai.request.max_tokens"), Some("256"));

	// -- Server
	assert_eq!(span.get("server.address"), Some("api.openai.com"));
	assert_eq!(span.get("server.port"), Some("443"));

	// -- Response
	assert_eq!(span.get("gen_ai.response.id"), Some("chatcmpl-123"));
	assert_eq!(span.get("gen_ai.response.model"), Some("gpt-4o-2024-08-06"));
	assert_eq!(span.get("gen_ai.response.finish_reasons"), Some(r#"["stop"]"#));

	// -- Usage
	assert_eq!(span.get("gen_ai.usage.input_tokens"), Some("10"));
	assert_eq!(span.get("gen_ai.usage.output_tokens"), Some("20"));

	// -- No error on the success path
	assert!(!span.has("error.type"));
	assert!(!span.has("otel.status_code"));
}

#[test]
fn test_otel_chat_span_content_absent_by_default() {
	let captured = capture(|| {
		let options_set = ChatOptionsSet::default();
		let chat_req = ChatRequest::from_system("be brief").append_message(ChatMessage::user("Hello"));
		let span = span::chat_request_span(&model(), &endpoint(), &options_set, &chat_req, false);
		span::record_chat_response(&span, &chat_response());
	});

	let span = captured.by_name("genai.chat");

	// Content capture is off by default → no sensitive content attributes.
	assert!(!span.has("gen_ai.system_instructions"));
	assert!(!span.has("gen_ai.input.messages"));
	assert!(!span.has("gen_ai.output.messages"));
	assert!(!span.has("gen_ai.tool.definitions"));
}

#[test]
fn test_otel_chat_span_records_error() {
	let captured = capture(|| {
		let options_set = ChatOptionsSet::default();
		let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello")]);
		let span = span::chat_request_span(&model(), &endpoint(), &options_set, &chat_req, false);
		span::record_error(&span, &crate::Error::Internal("boom".to_string()));
	});

	let span = captured.by_name("genai.chat");
	assert_eq!(span.get("otel.status_code"), Some("error"));
	assert_eq!(span.get("error.type"), Some("internal"));
}

// endregion: --- Chat span tests

// region:    --- Streaming span tests

#[test]
fn test_otel_chat_stream_span_records_ttfc_and_end() {
	use futures::StreamExt;

	let captured = capture(|| {
		let options_set = ChatOptionsSet::default();
		let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello")]);
		let span = span::chat_request_span(&model(), &endpoint(), &options_set, &chat_req, true);

		let inter = futures::stream::iter(vec![
			Ok(InterStreamEvent::Start),
			Ok(InterStreamEvent::Chunk("Hel".to_string())),
			Ok(InterStreamEvent::Chunk("lo".to_string())),
			Ok(InterStreamEvent::End(InterStreamEnd {
				captured_usage: Some(Usage {
					prompt_tokens: Some(5),
					completion_tokens: Some(7),
					total_tokens: Some(12),
					..Default::default()
				}),
				captured_stop_reason: Some(StopReason::Completed("stop".to_string())),
				..Default::default()
			})),
		]);

		let mut stream = ChatStream::from_inter_stream(inter).with_otel_span(span);
		futures::executor::block_on(async { while stream.next().await.is_some() {} });
	});

	let span = captured.by_name("genai.chat");
	assert_eq!(span.get("gen_ai.request.stream"), Some("true"));
	// Time-to-first-chunk recorded (value is timing-dependent; assert presence).
	assert!(span.has("gen_ai.response.time_to_first_chunk"));
	// End attributes captured.
	assert_eq!(span.get("gen_ai.usage.input_tokens"), Some("5"));
	assert_eq!(span.get("gen_ai.usage.output_tokens"), Some("7"));
	assert_eq!(span.get("gen_ai.response.finish_reasons"), Some(r#"["stop"]"#));
}

#[test]
fn test_otel_chat_stream_span_records_error() {
	use futures::StreamExt;

	let captured = capture(|| {
		let options_set = ChatOptionsSet::default();
		let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello")]);
		let span = span::chat_request_span(&model(), &endpoint(), &options_set, &chat_req, true);

		let inter = futures::stream::iter(vec![
			Ok(InterStreamEvent::Start),
			Err(crate::Error::Internal("stream boom".to_string())),
		]);

		let mut stream = ChatStream::from_inter_stream(inter).with_otel_span(span);
		futures::executor::block_on(async { while stream.next().await.is_some() {} });
	});

	let span = captured.by_name("genai.chat");
	assert_eq!(span.get("otel.status_code"), Some("error"));
	assert_eq!(span.get("error.type"), Some("internal"));
}

// endregion: --- Streaming span tests

// region:    --- Embeddings span tests

#[test]
fn test_otel_embeddings_span_attributes() {
	use crate::embed::{EmbedOptions, EmbedOptionsSet, EmbedResponse};

	let captured = capture(|| {
		let options = EmbedOptions::default().with_dimensions(512);
		let options_set = EmbedOptionsSet::new().with_request_options(Some(&options));
		let model = ModelIden::from_static(AdapterKind::OpenAI, "text-embedding-3-small");
		let span = span::embeddings_request_span(&model, &endpoint(), &options_set);

		let res = EmbedResponse {
			embeddings: vec![],
			model_iden: model.clone(),
			provider_model_iden: ModelIden::from_static(AdapterKind::OpenAI, "text-embedding-3-small"),
			usage: Usage {
				prompt_tokens: Some(8),
				total_tokens: Some(8),
				..Default::default()
			},
			captured_raw_body: None,
		};
		span::record_embed_response(&span, &res);
	});

	let span = captured.by_name("genai.embeddings");
	assert_eq!(span.get("otel.name"), Some("embeddings text-embedding-3-small"));
	assert_eq!(span.get("otel.kind"), Some("client"));
	assert_eq!(span.get("gen_ai.operation.name"), Some("embeddings"));
	assert_eq!(span.get("gen_ai.provider.name"), Some("openai"));
	assert_eq!(span.get("gen_ai.request.model"), Some("text-embedding-3-small"));
	assert_eq!(span.get("gen_ai.embeddings.dimension.count"), Some("512"));
	assert_eq!(span.get("gen_ai.usage.input_tokens"), Some("8"));
}

// endregion: --- Embeddings span tests

// region:    --- Helper builder tests

#[test]
fn test_otel_execute_tool_span() {
	use crate::chat::ToolCall;
	use serde_json::json;

	let captured = capture(|| {
		let tool_call = ToolCall {
			call_id: "call_42".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({ "location": "Paris" }),
			thought_signatures: None,
		};
		let _span = agent::execute_tool_span(&tool_call);
	});

	let span = captured.by_name("genai.execute_tool");
	assert_eq!(span.get("otel.name"), Some("execute_tool get_weather"));
	assert_eq!(span.get("otel.kind"), Some("internal"));
	assert_eq!(span.get("gen_ai.operation.name"), Some("execute_tool"));
	assert_eq!(span.get("gen_ai.tool.name"), Some("get_weather"));
	assert_eq!(span.get("gen_ai.tool.call.id"), Some("call_42"));
	assert_eq!(span.get("gen_ai.tool.type"), Some("function"));
	// Arguments are opt-in content → absent by default.
	assert!(!span.has("gen_ai.tool.call.arguments"));
}

#[test]
fn test_otel_agent_spans() {
	let captured = capture(|| {
		let _create = agent::AgentSpan::create_agent()
			.with_name("Greeter")
			.with_id("agent_1")
			.with_model("gpt-4o")
			.start();
		let _invoke = agent::AgentSpan::invoke_agent().with_name("Greeter").internal().start();
		let _workflow = agent::invoke_workflow_span("daily-report");
		let _plan = agent::plan_span(Some("Greeter"));
	});

	let create = captured.by_name("genai.agent");
	assert_eq!(create.get("gen_ai.operation.name"), Some("create_agent"));
	assert_eq!(create.get("otel.name"), Some("create_agent Greeter"));
	assert_eq!(create.get("otel.kind"), Some("client"));
	assert_eq!(create.get("gen_ai.agent.name"), Some("Greeter"));
	assert_eq!(create.get("gen_ai.agent.id"), Some("agent_1"));
	assert_eq!(create.get("gen_ai.request.model"), Some("gpt-4o"));

	let workflow = captured.by_name("genai.invoke_workflow");
	assert_eq!(workflow.get("otel.name"), Some("invoke_workflow daily-report"));
	assert_eq!(workflow.get("otel.kind"), Some("internal"));
	assert_eq!(workflow.get("gen_ai.workflow.name"), Some("daily-report"));

	let plan = captured.by_name("genai.plan");
	assert_eq!(plan.get("otel.name"), Some("plan Greeter"));
	assert_eq!(plan.get("gen_ai.agent.name"), Some("Greeter"));
}

#[test]
fn test_otel_evaluation_result_event() {
	let captured = capture(|| {
		events::EvalResult::new("Relevance")
			.with_score_value(4.0)
			.with_score_label("relevant")
			.with_response_id("chatcmpl-123")
			.emit();
	});

	let event = captured.by_name("genai.evaluation.result");
	assert_eq!(event.get("otel.name"), Some("gen_ai.evaluation.result"));
	assert_eq!(event.get("gen_ai.evaluation.name"), Some("Relevance"));
	assert_eq!(event.get("gen_ai.evaluation.score.value"), Some("4"));
	assert_eq!(event.get("gen_ai.evaluation.score.label"), Some("relevant"));
	assert_eq!(event.get("gen_ai.response.id"), Some("chatcmpl-123"));
}

// endregion: --- Helper builder tests
