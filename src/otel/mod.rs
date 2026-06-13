//! OpenTelemetry GenAI instrumentation (feature `otel`).
//!
//! This module instruments genai operations following the
//! [OpenTelemetry GenAI semantic conventions](https://github.com/open-telemetry/semantic-conventions-genai)
//! (status: Development; a spec snapshot lives under `docs/otel-genai-spec/`).
//!
//! ## How it works
//!
//! genai emits plain [`tracing`] spans whose field names follow the `gen_ai.*`
//! conventions, plus the `otel.kind` / `otel.name` / `otel.status_code` bridge
//! fields. To export them as OpenTelemetry spans, wire
//! [`tracing-opentelemetry`](https://docs.rs/tracing-opentelemetry) onto your
//! `tracing-subscriber` registry in the application — genai records to whatever
//! subscriber is installed and never owns the OTel pipeline itself.
//!
//! The `chat` (inference) and `embeddings` operations are instrumented
//! automatically. Agent / workflow / tool spans and the evaluation event have
//! no genai-internal trigger and are exposed as opt-in builder helpers
//! ([`agent`], [`events`]) for applications building higher-level flows on top
//! of genai.
//!
//! ## Content capture
//!
//! Prompt and response content (`gen_ai.input.messages`, `gen_ai.output.messages`,
//! `gen_ai.system_instructions`, `gen_ai.tool.definitions`) is **off by default**
//! because it is likely to contain sensitive / PII data. Enable it by setting the
//! [`CAPTURE_CONTENT_ENV`](attrs::CAPTURE_CONTENT_ENV) environment variable.
//!
//! ## Limitations
//!
//! - Spans only — the spec's metric instruments are not emitted (usage/duration
//!   are available as span attributes; backends can derive metrics from spans).
//! - Array-valued attributes (`finish_reasons`, `stop_sequences`) and message
//!   content are encoded as JSON strings, which the spec permits on spans.

pub mod conventions;

pub mod agent;
pub mod events;

pub(crate) mod attrs;
pub(crate) mod error;
pub(crate) mod span;

#[cfg(test)]
mod tests;

// -- Public re-exports
pub use attrs::{CAPTURE_CONTENT_ENV, capture_content};
pub use conventions::provider_name;
pub use error::error_type;
