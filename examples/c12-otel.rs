//! OpenTelemetry GenAI instrumentation example (feature `otel`).
//!
//! Run with:
//! ```sh
//! OPENAI_API_KEY=... cargo run --example c12-otel --features otel
//! ```
//!
//! To also capture prompt/response content on the spans (off by default, since
//! it may contain sensitive data), set the standard opt-in env var:
//! ```sh
//! OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT=true \
//!   OPENAI_API_KEY=... cargo run --example c12-otel --features otel
//! ```
//!
//! genai emits `gen_ai.*` `tracing` spans following the OpenTelemetry GenAI
//! semantic conventions. This example installs a plain `tracing-subscriber`
//! `fmt` subscriber and prints each span (with its attributes) when it closes,
//! so you can see what gets recorded without standing up an OTel collector.
//!
//! To export real OpenTelemetry spans, replace the `fmt` layer below with a
//! `tracing-opentelemetry` layer in your application, e.g.:
//! ```ignore
//! use tracing_subscriber::prelude::*;
//! let tracer = /* your OpenTelemetry tracer (OTLP, stdout, ...) */;
//! tracing_subscriber::registry()
//!     .with(tracing_opentelemetry::layer().with_tracer(tracer))
//!     .init();
//! ```
//! genai records to whatever subscriber is installed; it never owns the OTel
//! pipeline itself.

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

const MODEL: &str = "gpt-5.4-mini";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Content capture (`gen_ai.input.messages` / `output.messages` / ...) is opt-in
	// via the `OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT` env var — see the
	// run command in the module docs above.

	// -- Print spans (and their `gen_ai.*` fields) as they close.
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=info"))
		.with_span_events(FmtSpan::CLOSE)
		.init();

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one short sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);

	// -- Non-streaming: produces a `chat <model>` span with usage + response attributes.
	println!("\n=== exec_chat ===");
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;
	println!("Response: {}", chat_res.first_text().unwrap_or("NO ANSWER"));

	// -- Streaming: the span stays open for the whole stream and records
	//    `gen_ai.response.time_to_first_chunk` plus the captured end attributes.
	println!("\n=== exec_chat_stream ===");
	let mut chat_stream = client.exec_chat_stream(MODEL, chat_req, None).await?.stream;
	use futures::StreamExt;
	use genai::chat::ChatStreamEvent;
	while let Some(event) = chat_stream.next().await {
		if let ChatStreamEvent::Chunk(chunk) = event? {
			print!("{}", chunk.content);
		}
	}
	println!();

	Ok(())
}
