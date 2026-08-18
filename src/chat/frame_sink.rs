//! Raw provider frame callbacks.
//!
//! Used internally for debugging and as a convenient escape hatch without
//! the need for changing the internals of the library. exec_chat_stream provies
//! a normalized view of providers but hides many of the details that may be required
//! for billing. For e.g., a gemini grounding search costs $0.014 for search. There
//! are similar items like Anthropic's server_tool_use or OpenAI web_search_call.
//!
//! A [`ChatFrameSink`] set on [`ChatOptions`](crate::chat::ChatOptions) receives every wire
//! frame as it is decoded, so callers can aggregate or log whatever they need. The library
//!
//! The sink is invoked for both exec paths — once with the whole body for `exec_chat`
//! (`FrameCtx.streaming == false`), and per frame for `exec_chat_stream`.

use crate::ModelIden;
use crate::chat::StreamEnd;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

// region:    --- RawFrame

/// An owned provider frame, as produced by [`RawFrameRef::to_owned_frame`].
///
/// Only materialized when a sink asks for it; the library never builds these on its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFrame {
	/// 0-based ordinal within the stream.
	pub index: u32,

	/// Provider event name when the transport has one:
	/// SSE `event:` line, or the Bedrock event-stream `:event-type` header.
	/// `None` for line-delimited JSON transports (Ollama, Cohere).
	pub event: Option<String>,

	/// The payload, parsed as JSON when possible.
	pub data: RawFrameData,

	/// Microseconds since the stream was issued.
	pub elapsed_us: u64,
}

/// Payload of a [`RawFrame`]. `Text` preserves frames that are not JSON
/// (`[DONE]` sentinels, keep-alives, malformed payloads).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawFrameData {
	/// A payload that parsed as JSON.
	Json(Value),
	/// A payload that did not parse as JSON, kept verbatim.
	Text(String),
}

impl RawFrameData {
	/// The payload as JSON, or `None` when it did not parse.
	pub fn as_json(&self) -> Option<&Value> {
		match self {
			RawFrameData::Json(value) => Some(value),
			RawFrameData::Text(_) => None,
		}
	}

	/// The payload as raw text, or `None` when it parsed as JSON.
	pub fn as_text(&self) -> Option<&str> {
		match self {
			RawFrameData::Json(_) => None,
			RawFrameData::Text(text) => Some(text),
		}
	}
}

// endregion: --- RawFrame

// region:    --- RawFrameRef

/// A borrowed view of one wire frame. Nothing is parsed or allocated unless the sink asks.
#[derive(Debug, Clone, Copy)]
pub struct RawFrameRef<'a> {
	/// 0-based ordinal within the stream.
	pub index: u32,

	/// Provider event name, when the transport has one (see [`RawFrame::event`]).
	pub event: Option<&'a str>,

	/// The exact payload text, unparsed.
	pub data: &'a str,

	/// Microseconds since the stream was issued.
	pub elapsed_us: u64,
}

impl RawFrameRef<'_> {
	/// Parses the payload as JSON. Returns `None` for non-JSON frames.
	pub fn json(&self) -> Option<Value> {
		serde_json::from_str(self.data).ok()
	}

	/// Materializes an owned [`RawFrame`], parsing the payload as JSON when possible.
	pub fn to_owned_frame(&self) -> RawFrame {
		let data = match serde_json::from_str::<Value>(self.data) {
			Ok(value) => RawFrameData::Json(value),
			Err(_) => RawFrameData::Text(self.data.to_string()),
		};

		RawFrame {
			index: self.index,
			event: self.event.map(String::from),
			data,
			elapsed_us: self.elapsed_us,
		}
	}
}

// endregion: --- RawFrameRef

// region:    --- FrameCtx

/// Identifies the stream a frame belongs to.
///
/// A sink set as a client default sees every concurrent stream through the same `&self`,
/// so accumulating sinks bucket by `stream_id`.
#[derive(Debug, Clone)]
pub struct FrameCtx {
	/// Process-wide identifier, assigned per streaming call.
	pub stream_id: u64,

	/// The resolved model for this call.
	pub model_iden: ModelIden,
}

impl FrameCtx {
	pub(crate) fn new(model_iden: ModelIden) -> Self {
		static NEXT_STREAM_ID: AtomicU64 = AtomicU64::new(1);

		Self {
			stream_id: NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed),
			model_iden,
		}
	}
}

// endregion: --- FrameCtx

// region:    --- ChatFrameSink

/// Receives raw provider frames as they are decoded.
///
/// Streaming only — `exec_chat` ignores any sink, since `ChatResponse.captured_raw_body`
/// already carries the whole body.
///
/// - No `.await`, no blocking I/O, and no long lock hold: a slow sink stalls the stream.
///   Push heavy work elsewhere (see [`ChannelSink`]).
/// - Frames arrive in wire order per `FrameCtx.stream_id`; there is no ordering guarantee
///   across streams sharing one sink.
/// - `on_end` is not called when a stream is dropped before completion.
pub trait ChatFrameSink: Send + Sync {
	/// Called once per wire frame.
	fn on_frame(&self, ctx: &FrameCtx, frame: RawFrameRef<'_>);

	/// Called once when the stream completes normally, with the terminal `StreamEnd`.
	fn on_end(&self, _ctx: &FrameCtx, _end: &StreamEnd) {}

	/// Called when the stream fails. A mid-stream failure ends the stream without an
	/// `End` event, so this is the only notification a sink gets.
	fn on_error(&self, _ctx: &FrameCtx, _err: &crate::Error) {}
}

impl std::fmt::Debug for dyn ChatFrameSink {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ChatFrameSink")
	}
}

// endregion: --- ChatFrameSink

// region:    --- Provided Sinks

/// Wraps a closure as a [`ChatFrameSink`] (frames only).
///
/// Usually built through `ChatOptions::with_raw_frame_fn`.
pub struct FnSink<F>(F);

impl<F> FnSink<F>
where
	F: Fn(&FrameCtx, RawFrameRef<'_>) + Send + Sync + 'static,
{
	/// Creates a sink from a per-frame closure.
	pub fn new(f: F) -> Self {
		Self(f)
	}
}

impl<F> ChatFrameSink for FnSink<F>
where
	F: Fn(&FrameCtx, RawFrameRef<'_>) + Send + Sync + 'static,
{
	fn on_frame(&self, ctx: &FrameCtx, frame: RawFrameRef<'_>) {
		(self.0)(ctx, frame)
	}
}

/// Buffers every frame in memory.
///
/// This is the full-capture escape hatch. It is unbounded **by the caller's choice** — the
/// library has no opinion on how much of a stream to retain, so prefer an aggregating sink
/// for long-running streams.
#[derive(Debug, Default)]
pub struct CollectorSink {
	frames: std::sync::Mutex<Vec<RawFrame>>,
}

impl CollectorSink {
	/// Creates an empty collector.
	pub fn new() -> Self {
		Self::default()
	}

	/// Clones out the frames collected so far.
	pub fn frames(&self) -> Vec<RawFrame> {
		self.frames.lock().map(|frames| frames.clone()).unwrap_or_default()
	}

	/// Number of frames collected so far.
	pub fn len(&self) -> usize {
		self.frames.lock().map(|frames| frames.len()).unwrap_or(0)
	}

	/// True when no frame has been collected.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Takes the collected frames, leaving the collector empty.
	pub fn take(&self) -> Vec<RawFrame> {
		self.frames
			.lock()
			.map(|mut frames| std::mem::take(&mut *frames))
			.unwrap_or_default()
	}
}

impl ChatFrameSink for CollectorSink {
	fn on_frame(&self, _ctx: &FrameCtx, frame: RawFrameRef<'_>) {
		if let Ok(mut frames) = self.frames.lock() {
			frames.push(frame.to_owned_frame());
		}
	}
}

/// Forwards frames to a `tokio` channel for processing off the stream's task.
///
/// Uses `try_send`, so a full channel drops frames rather than stalling the stream;
/// [`ChannelSink::dropped`] reports how many.
#[derive(Debug)]
pub struct ChannelSink {
	tx: tokio::sync::mpsc::Sender<RawFrame>,
	dropped: AtomicU64,
}

impl ChannelSink {
	/// Creates a sink forwarding to `tx`.
	pub fn new(tx: tokio::sync::mpsc::Sender<RawFrame>) -> Self {
		Self {
			tx,
			dropped: AtomicU64::new(0),
		}
	}

	/// Number of frames dropped because the channel was full or closed.
	pub fn dropped(&self) -> u64 {
		self.dropped.load(Ordering::Relaxed)
	}
}

impl ChatFrameSink for ChannelSink {
	fn on_frame(&self, _ctx: &FrameCtx, frame: RawFrameRef<'_>) {
		if self.tx.try_send(frame.to_owned_frame()).is_err() {
			self.dropped.fetch_add(1, Ordering::Relaxed);
		}
	}
}

// endregion: --- Provided Sinks

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::adapter::AdapterKind;
	use std::sync::Arc;

	fn frame_ref<'a>(index: u32, event: Option<&'a str>, data: &'a str) -> RawFrameRef<'a> {
		RawFrameRef {
			index,
			event,
			data,
			elapsed_us: 42,
		}
	}

	fn ctx() -> FrameCtx {
		FrameCtx::new(ModelIden::new(AdapterKind::OpenAI, "gpt-test"))
	}

	#[test]
	fn test_frame_sink_owned_frame_json() -> Result<()> {
		// -- Exec
		let frame = frame_ref(1, Some("content_block_delta"), r#"{"a":1}"#).to_owned_frame();

		// -- Check
		assert_eq!(frame.index, 1);
		assert_eq!(frame.event.as_deref(), Some("content_block_delta"));
		assert_eq!(
			frame.data.as_json().and_then(|v| v.get("a")).and_then(|v| v.as_i64()),
			Some(1)
		);

		Ok(())
	}

	#[test]
	fn test_frame_sink_owned_frame_text_fallback() -> Result<()> {
		// -- Exec
		let frame = frame_ref(0, None, "[DONE]").to_owned_frame();

		// -- Check
		assert_eq!(frame.data.as_text(), Some("[DONE]"));
		assert!(frame.data.as_json().is_none());

		Ok(())
	}

	#[test]
	fn test_frame_sink_raw_frame_serde_roundtrip() -> Result<()> {
		// -- Setup & Fixtures
		let frames = vec![
			frame_ref(0, Some("message"), r#"{"a":1}"#).to_owned_frame(),
			frame_ref(1, None, "[DONE]").to_owned_frame(),
		];

		// -- Exec
		let json = serde_json::to_string(&frames)?;
		let back: Vec<RawFrame> = serde_json::from_str(&json)?;

		// -- Check
		assert_eq!(back.len(), 2);
		assert!(back[0].data.as_json().is_some(), "first frame should stay json");
		assert_eq!(
			back[1].data.as_text(),
			Some("[DONE]"),
			"text frame should not become json"
		);

		Ok(())
	}

	#[test]
	fn test_frame_sink_collector_collects() -> Result<()> {
		// -- Setup & Fixtures
		let sink = CollectorSink::new();
		let ctx = ctx();

		// -- Exec
		sink.on_frame(&ctx, frame_ref(0, Some("a"), r#"{"n":1}"#));
		sink.on_frame(&ctx, frame_ref(1, Some("b"), r#"{"n":2}"#));

		// -- Check
		assert_eq!(sink.len(), 2);
		let frames = sink.take();
		assert_eq!(frames.iter().map(|f| f.index).collect::<Vec<_>>(), vec![0, 1]);
		assert!(sink.is_empty(), "take should leave the collector empty");

		Ok(())
	}

	#[test]
	fn test_frame_sink_fn_sink_calls_closure() -> Result<()> {
		// -- Setup & Fixtures
		let count = Arc::new(AtomicU64::new(0));
		let count_clone = count.clone();
		let sink = FnSink::new(move |_ctx: &FrameCtx, _frame: RawFrameRef<'_>| {
			count_clone.fetch_add(1, Ordering::Relaxed);
		});

		// -- Exec
		sink.on_frame(&ctx(), frame_ref(0, None, "{}"));

		// -- Check
		assert_eq!(count.load(Ordering::Relaxed), 1);

		Ok(())
	}
}

// endregion: --- Tests
