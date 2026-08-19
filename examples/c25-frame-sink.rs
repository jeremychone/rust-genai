//! Raw frame sink: aggregate provider data the normalized stream events do not model.
//!
//! Server-side tool calls never reach the client as tool calls, so the only place a web
//! search shows up is the wire payload — `groundingMetadata` for Gemini, `server_tool_use`
//! for Anthropic, `web_search_call` output items for the OpenAI Responses API. Since these
//! are billed per request at different rates per provider, counting them needs the frames.
//!
//! `ChatFrameSink` gets every frame as it is decoded, so the sink can aggregate (as below)
//! instead of the library buffering the stream.

use futures::StreamExt;
use genai::chat::{
	ChatFrameSink, ChatOptions, ChatRequest, ChatStreamEvent, FrameCtx, RawFrameRef, StreamEnd, Tool, ToolName,
	WebSearchConfig,
};
use genai::{Client, Error};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const MODEL: &str = "gemini-3-flash-preview";

/// Counts server-side search activity and keeps a little latency profile.
/// Bounded memory: it accumulates counters, never the frames themselves.
#[derive(Debug, Default)]
struct SearchCounter {
	frames: AtomicUsize,
	search_queries: AtomicUsize,
	first_frame_us: AtomicU64,
}

impl ChatFrameSink for SearchCounter {
	fn on_frame(&self, _ctx: &FrameCtx, frame: RawFrameRef<'_>) {
		if self.frames.fetch_add(1, Ordering::Relaxed) == 0 {
			self.first_frame_us.store(frame.elapsed_us, Ordering::Relaxed);
		}

		if !frame.data.contains("groundingMetadata") {
			return;
		}

		if let Some(value) = frame.json()
			&& let Some(queries) = value
				.pointer("/candidates/0/groundingMetadata/webSearchQueries")
				.and_then(|queries| queries.as_array())
		{
			self.search_queries.fetch_add(queries.len(), Ordering::Relaxed);
		}
	}

	fn on_end(&self, ctx: &FrameCtx, _end: &StreamEnd) {
		println!("-- sink: stream #{} complete", ctx.stream_id);
	}

	fn on_error(&self, _ctx: &FrameCtx, err: &Error) {
		// The only notification for a mid-stream failure: it ends the stream with no End event.
		println!("-- sink: stream failed: {err}");
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "What was announced in the most recent Rust release? Keep it to 3 bullet points.";

	// -- Setup the sink; keep the Arc to read what it accumulated
	let counter = Arc::new(SearchCounter::default());
	let sink: Arc<dyn ChatFrameSink> = counter.clone();
	let options = ChatOptions::default().with_capture_usage(true).with_raw_frame_sink_arc(sink);

	let client = Client::default();
	let chat_req = ChatRequest::from_user(question)
		.append_tool(Tool::new(ToolName::WebSearch).with_config(WebSearchConfig::default()));

	// -- Exec
	println!("=== Question:\n{question}\n\n=== Answer:");
	let chat_res = client.exec_chat_stream(MODEL, chat_req, Some(&options)).await?;
	let mut stream = chat_res.stream;

	while let Some(event) = stream.next().await {
		match event? {
			ChatStreamEvent::Chunk(chunk) => print!("{}", chunk.content),
			ChatStreamEvent::End(end) => {
				if let Some(usage) = end.captured_usage.as_ref() {
					println!("\n\n=== Usage:\n{usage:?}");
				}
			}
			_ => (),
		}
	}

	// -- What the normalized events could not tell us
	println!(
		"\n=== Sink:\nframes: {}\nserver-side search queries: {}\ntime to first frame: {}ms",
		counter.frames.load(Ordering::Relaxed),
		counter.search_queries.load(Ordering::Relaxed),
		counter.first_frame_us.load(Ordering::Relaxed) / 1000
	);

	Ok(())
}
