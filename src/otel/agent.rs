//! Opt-in span builders for GenAI agent / workflow / tool operations.
//!
//! genai itself performs no agent, workflow, planning, or tool execution — it
//! is a provider client. These builders exist so applications building such
//! flows on top of genai can emit spec-compliant
//! ([`gen-ai-agent-spans`](https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-agent-spans.md))
//! spans without hand-rolling the `gen_ai.*` field names.
//!
//! Each builder returns a [`tracing::Span`]; enter it (`let _g = span.enter();`)
//! around your own logic. Mark failures with [`set_error`] and, for tool spans,
//! record the result with [`ExecuteToolSpan::record_result`].

use crate::chat::ToolCall;
use crate::otel::conventions as c;
use crate::otel::{attrs, error as otel_error};
use tracing::Span;
use tracing::field::Empty;

// region:    --- Shared helpers

/// Marks a helper span as errored: sets `otel.status_code = error` and the given
/// low-cardinality `error.type`.
pub fn set_error(span: &Span, error_type: impl AsRef<str>) {
	span.record(c::OTEL_STATUS_CODE, c::STATUS_ERROR);
	span.record(c::ERROR_TYPE, error_type.as_ref());
}

/// Marks a helper span as errored from a genai [`Error`](crate::Error), deriving
/// the `error.type` the same way the auto-instrumented spans do.
pub fn set_genai_error(span: &Span, error: &crate::Error) {
	set_error(span, otel_error::error_type(error));
}

// endregion: --- Shared helpers

// region:    --- Execute tool span

/// Builder for an `execute_tool` span (kind `INTERNAL`).
#[derive(Debug, Clone, Default)]
pub struct ExecuteToolSpan {
	name: String,
	call_id: Option<String>,
	description: Option<String>,
	tool_type: Option<String>,
	arguments: Option<String>,
}

impl ExecuteToolSpan {
	/// Starts a builder for the tool with the given name (`gen_ai.tool.name`).
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			..Default::default()
		}
	}

	/// Sets `gen_ai.tool.call.id`.
	pub fn with_call_id(mut self, call_id: impl Into<String>) -> Self {
		self.call_id = Some(call_id.into());
		self
	}

	/// Sets `gen_ai.tool.description`.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Sets `gen_ai.tool.type` (e.g. `function`, `extension`, `datastore`).
	pub fn with_tool_type(mut self, tool_type: impl Into<String>) -> Self {
		self.tool_type = Some(tool_type.into());
		self
	}

	/// Sets the tool-call arguments (opt-in content; recorded only when content
	/// capture is enabled — see [`capture_content`](crate::otel::capture_content)).
	pub fn with_arguments(mut self, arguments: impl Into<String>) -> Self {
		self.arguments = Some(arguments.into());
		self
	}

	/// Builds and returns the span, recording the configured attributes.
	pub fn start(&self) -> Span {
		let span_name = format!("{} {}", c::OP_EXECUTE_TOOL, self.name);
		let span = tracing::info_span!(
			"genai.execute_tool",
			"otel.name" = span_name.as_str(),
			"otel.kind" = c::KIND_INTERNAL,
			"otel.status_code" = Empty,
			"error.type" = Empty,
			"gen_ai.operation.name" = c::OP_EXECUTE_TOOL,
			"gen_ai.tool.name" = self.name.as_str(),
			"gen_ai.tool.call.id" = Empty,
			"gen_ai.tool.description" = Empty,
			"gen_ai.tool.type" = Empty,
			"gen_ai.tool.call.arguments" = Empty,
			"gen_ai.tool.call.result" = Empty,
		);
		if let Some(call_id) = &self.call_id {
			span.record(c::TOOL_CALL_ID, call_id.as_str());
		}
		if let Some(description) = &self.description {
			span.record(c::TOOL_DESCRIPTION, description.as_str());
		}
		if let Some(tool_type) = &self.tool_type {
			span.record(c::TOOL_TYPE, tool_type.as_str());
		}
		if attrs::capture_content()
			&& let Some(arguments) = &self.arguments
		{
			span.record(c::TOOL_CALL_ARGUMENTS, arguments.as_str());
		}
		span
	}

	/// Records the tool-call result on the span (opt-in content; recorded only
	/// when content capture is enabled).
	pub fn record_result(span: &Span, result: impl AsRef<str>) {
		if attrs::capture_content() {
			span.record(c::TOOL_CALL_RESULT, result.as_ref());
		}
	}
}

/// Convenience: builds an [`ExecuteToolSpan`] from a genai [`ToolCall`]
/// (`function` type), serializing the arguments for opt-in content capture.
pub fn execute_tool_span(tool_call: &ToolCall) -> Span {
	let mut builder = ExecuteToolSpan::new(tool_call.fn_name.clone())
		.with_call_id(tool_call.call_id.clone())
		.with_tool_type("function");
	if let Ok(arguments) = serde_json::to_string(&tool_call.fn_arguments) {
		builder = builder.with_arguments(arguments);
	}
	builder.start()
}

// endregion: --- Execute tool span

// region:    --- Agent span (create_agent / invoke_agent)

/// Builder for `create_agent` / `invoke_agent` spans.
#[derive(Debug, Clone)]
pub struct AgentSpan {
	operation: &'static str,
	kind: &'static str,
	name: Option<String>,
	id: Option<String>,
	description: Option<String>,
	provider: Option<String>,
	model: Option<String>,
}

impl AgentSpan {
	/// Builder for a `create_agent` span (kind `CLIENT`).
	pub fn create_agent() -> Self {
		Self::new(c::OP_CREATE_AGENT, c::KIND_CLIENT)
	}

	/// Builder for an `invoke_agent` span (kind `CLIENT`).
	///
	/// Use [`AgentSpan::internal`] to switch to `INTERNAL` when invoking an agent
	/// running in the same process.
	pub fn invoke_agent() -> Self {
		Self::new(c::OP_INVOKE_AGENT, c::KIND_CLIENT)
	}

	fn new(operation: &'static str, kind: &'static str) -> Self {
		Self {
			operation,
			kind,
			name: None,
			id: None,
			description: None,
			provider: None,
			model: None,
		}
	}

	/// Uses `INTERNAL` span kind instead of the default `CLIENT`.
	pub fn internal(mut self) -> Self {
		self.kind = c::KIND_INTERNAL;
		self
	}

	/// Sets `gen_ai.agent.name`.
	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		self.name = Some(name.into());
		self
	}

	/// Sets `gen_ai.agent.id`.
	pub fn with_id(mut self, id: impl Into<String>) -> Self {
		self.id = Some(id.into());
		self
	}

	/// Sets `gen_ai.agent.description`.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Sets `gen_ai.provider.name`.
	pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
		self.provider = Some(provider.into());
		self
	}

	/// Sets `gen_ai.request.model`.
	pub fn with_model(mut self, model: impl Into<String>) -> Self {
		self.model = Some(model.into());
		self
	}

	/// Builds and returns the span.
	pub fn start(&self) -> Span {
		// Span name is `{operation} {agent.name}` when the name is available.
		let span_name = match &self.name {
			Some(name) => format!("{} {name}", self.operation),
			None => self.operation.to_string(),
		};
		let span = tracing::info_span!(
			"genai.agent",
			"otel.name" = span_name.as_str(),
			"otel.kind" = self.kind,
			"otel.status_code" = Empty,
			"error.type" = Empty,
			"gen_ai.operation.name" = self.operation,
			"gen_ai.agent.name" = Empty,
			"gen_ai.agent.id" = Empty,
			"gen_ai.agent.description" = Empty,
			"gen_ai.provider.name" = Empty,
			"gen_ai.request.model" = Empty,
		);
		if let Some(name) = &self.name {
			span.record(c::AGENT_NAME, name.as_str());
		}
		if let Some(id) = &self.id {
			span.record(c::AGENT_ID, id.as_str());
		}
		if let Some(description) = &self.description {
			span.record(c::AGENT_DESCRIPTION, description.as_str());
		}
		if let Some(provider) = &self.provider {
			span.record(c::PROVIDER_NAME, provider.as_str());
		}
		if let Some(model) = &self.model {
			span.record(c::REQUEST_MODEL, model.as_str());
		}
		span
	}
}

// endregion: --- Agent span (create_agent / invoke_agent)

// region:    --- Workflow span

/// Creates an `invoke_workflow` span (kind `INTERNAL`).
pub fn invoke_workflow_span(workflow_name: impl AsRef<str>) -> Span {
	let workflow_name = workflow_name.as_ref();
	let span_name = format!("{} {workflow_name}", c::OP_INVOKE_WORKFLOW);
	let span = tracing::info_span!(
		"genai.invoke_workflow",
		"otel.name" = span_name.as_str(),
		"otel.kind" = c::KIND_INTERNAL,
		"otel.status_code" = Empty,
		"error.type" = Empty,
		"gen_ai.operation.name" = c::OP_INVOKE_WORKFLOW,
		"gen_ai.workflow.name" = workflow_name,
	);
	span
}

// endregion: --- Workflow span

// region:    --- Plan span

/// Creates a `plan` span (kind `INTERNAL`). The optional agent name shapes the
/// span name (`plan {agent.name}` when provided).
pub fn plan_span(agent_name: Option<&str>) -> Span {
	let span_name = match agent_name {
		Some(name) => format!("{} {name}", c::OP_PLAN),
		None => c::OP_PLAN.to_string(),
	};
	let span = tracing::info_span!(
		"genai.plan",
		"otel.name" = span_name.as_str(),
		"otel.kind" = c::KIND_INTERNAL,
		"otel.status_code" = Empty,
		"error.type" = Empty,
		"gen_ai.operation.name" = c::OP_PLAN,
		"gen_ai.agent.name" = Empty,
	);
	if let Some(name) = agent_name {
		span.record(c::AGENT_NAME, name);
	}
	span
}

// endregion: --- Plan span
