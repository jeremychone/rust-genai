//! Opt-in emitters for the GenAI
//! [events](https://github.com/open-telemetry/semantic-conventions-genai/blob/main/docs/gen-ai/gen-ai-events.md).
//!
//! ## Representation in the tracing bridge
//!
//! The spec models these as log-based events. In genai's tracing-bridge a
//! `tracing` *event* cannot carry conditionally-present fields (its field set is
//! fixed at the macro call site with no post-hoc `record`). So each emitter here
//! produces a **point-in-time span** (created and immediately closed) carrying
//! the event's attributes and parented to the currently-active operation span.
//! Through `tracing-opentelemetry` this surfaces as a zero-duration span with
//! the same attributes and parenting the spec prescribes for the event.
//!
//! The inference request/response content (the
//! `gen_ai.client.inference.operation.details` event) is, by default, captured
//! directly on the auto-instrumented `chat` span when content capture is enabled
//! — the spec explicitly allows recording content on the span as an alternative
//! to the event — so a separate emitter is not required for that signal.

use crate::otel::conventions as c;
use tracing::field::Empty;

// region:    --- Evaluation result

/// Result of evaluating a GenAI output, emitted as a `gen_ai.evaluation.result`
/// signal (see [module docs](self) for how it surfaces in the tracing bridge).
///
/// Build it, set the optional fields, and call [`EvalResult::emit`] while the
/// span being evaluated is active so the signal is correlated to it.
#[derive(Debug, Clone)]
pub struct EvalResult {
	name: String,
	score_value: Option<f64>,
	score_label: Option<String>,
	explanation: Option<String>,
	response_id: Option<String>,
	error_type: Option<String>,
}

impl EvalResult {
	/// Starts an evaluation result with the (required) evaluation metric name
	/// (`gen_ai.evaluation.name`, e.g. `Relevance`).
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			score_value: None,
			score_label: None,
			explanation: None,
			response_id: None,
			error_type: None,
		}
	}

	/// Sets `gen_ai.evaluation.score.value`.
	pub fn with_score_value(mut self, value: f64) -> Self {
		self.score_value = Some(value);
		self
	}

	/// Sets `gen_ai.evaluation.score.label` (e.g. `relevant`, `pass`).
	pub fn with_score_label(mut self, label: impl Into<String>) -> Self {
		self.score_label = Some(label.into());
		self
	}

	/// Sets `gen_ai.evaluation.explanation`.
	pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
		self.explanation = Some(explanation.into());
		self
	}

	/// Sets `gen_ai.response.id` to correlate the evaluation with the completion
	/// it evaluates (useful when the operation span id is not available).
	pub fn with_response_id(mut self, response_id: impl Into<String>) -> Self {
		self.response_id = Some(response_id.into());
		self
	}

	/// Sets `error.type` when the evaluation itself ended in an error.
	pub fn with_error_type(mut self, error_type: impl Into<String>) -> Self {
		self.error_type = Some(error_type.into());
		self
	}

	/// Emits the evaluation result signal.
	pub fn emit(&self) {
		let span = tracing::info_span!(
			"genai.evaluation.result",
			"otel.name" = "gen_ai.evaluation.result",
			"otel.kind" = c::KIND_INTERNAL,
			"otel.status_code" = Empty,
			"gen_ai.evaluation.name" = self.name.as_str(),
			"gen_ai.evaluation.score.value" = Empty,
			"gen_ai.evaluation.score.label" = Empty,
			"gen_ai.evaluation.explanation" = Empty,
			"gen_ai.response.id" = Empty,
			"error.type" = Empty,
		);
		if let Some(score_value) = self.score_value {
			span.record(c::EVALUATION_SCORE_VALUE, score_value);
		}
		if let Some(score_label) = &self.score_label {
			span.record(c::EVALUATION_SCORE_LABEL, score_label.as_str());
		}
		if let Some(explanation) = &self.explanation {
			span.record(c::EVALUATION_EXPLANATION, explanation.as_str());
		}
		if let Some(response_id) = &self.response_id {
			span.record(c::RESPONSE_ID, response_id.as_str());
		}
		if let Some(error_type) = &self.error_type {
			span.record(c::OTEL_STATUS_CODE, c::STATUS_ERROR);
			span.record(c::ERROR_TYPE, error_type.as_str());
		}
		// Created and dropped here → a point-in-time span (see module docs).
	}
}

// endregion: --- Evaluation result
