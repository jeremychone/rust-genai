use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatMessage, ContentPart, MessageContent, StopReason, ToolCall, Usage};
use crate::webc::FrameTap;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

type InterStreamType = Pin<Box<dyn Stream<Item = crate::Result<InterStreamEvent>> + Send>>;

/// A stream of chat events produced by a streaming chat request.
pub struct ChatStream {
	inter_stream: InterStreamType,

	/// Terminal hooks for the user `ChatFrameSink`.
	/// Per-frame calls happen in the transport; `on_end` fires here, where the public
	/// `StreamEnd` is built, and `on_error` fires on the error path. The tap latches, so
	/// only the first of the two reaches the sink.
	frame_tap: Option<FrameTap>,

	/// OTel span state for the streaming operation (feature `otel`).
	/// Set by `Client::exec_chat_stream` so the span covers the full stream
	/// lifetime and records time-to-first-chunk and the captured end attributes.
	#[cfg(feature = "otel")]
	otel: Option<OtelStreamState>,
}

/// Per-stream OTel state: the operation span, the issuance instant (for
/// time-to-first-chunk), and whether the first chunk was already recorded.
#[cfg(feature = "otel")]
struct OtelStreamState {
	span: tracing::Span,
	start: std::time::Instant,
	first_chunk_recorded: bool,
}

impl ChatStream {
	pub(crate) fn new(inter_stream: InterStreamType) -> Self {
		ChatStream {
			inter_stream,
			frame_tap: None,
			#[cfg(feature = "otel")]
			otel: None,
		}
	}

	pub(crate) fn from_inter_stream<T>(inter_stream: T) -> Self
	where
		T: Stream<Item = crate::Result<InterStreamEvent>> + Send + 'static,
	{
		let boxed_stream: InterStreamType = Box::pin(inter_stream);
		ChatStream::new(boxed_stream)
	}

	/// Attaches the frame tap cloned from the streamer, so the terminal sink hooks fire once
	/// the public `StreamEnd` exists (or once the stream fails).
	pub(crate) fn with_frame_tap(mut self, frame_tap: Option<FrameTap>) -> Self {
		self.frame_tap = frame_tap;
		self
	}

	/// Attaches the OTel operation span, starting the time-to-first-chunk clock.
	/// The span stays open until the stream is fully consumed or dropped.
	#[cfg(feature = "otel")]
	pub(crate) fn with_otel_span(mut self, span: tracing::Span) -> Self {
		self.otel = Some(OtelStreamState {
			span,
			start: std::time::Instant::now(),
			first_chunk_recorded: false,
		});
		self
	}
}

// region:    --- Stream Impl

impl Stream for ChatStream {
	type Item = crate::Result<ChatStreamEvent>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		match Pin::new(&mut this.inter_stream).poll_next(cx) {
			Poll::Ready(Some(Ok(event))) => {
				let chat_event = match event {
					InterStreamEvent::Start => ChatStreamEvent::Start,
					InterStreamEvent::Chunk(content) => ChatStreamEvent::Chunk(StreamChunk { content }),
					InterStreamEvent::ReasoningChunk(content) => {
						ChatStreamEvent::ReasoningChunk(StreamChunk { content })
					}
					InterStreamEvent::ThoughtSignatureChunk(content) => {
						ChatStreamEvent::ThoughtSignatureChunk(StreamChunk { content })
					}
					InterStreamEvent::ToolCallChunk(tool_call) => {
						ChatStreamEvent::ToolCallChunk(ToolChunk { tool_call })
					}
					InterStreamEvent::Heartbeat => ChatStreamEvent::Heartbeat,
					InterStreamEvent::End(inter_end) => ChatStreamEvent::End(inter_end.into()),
				};

				// -- Frame sink: terminal hook, once, with the public StreamEnd.
				if let Some(frame_tap) = this.frame_tap.as_ref()
					&& let ChatStreamEvent::End(stream_end) = &chat_event
				{
					frame_tap.on_stream_end(stream_end);
				}

				// -- OTel: record time-to-first-chunk on the first content chunk, and the
				//          captured usage/finish/content on the end event.
				#[cfg(feature = "otel")]
				if let Some(otel) = this.otel.as_mut() {
					match &chat_event {
						ChatStreamEvent::Chunk(_) | ChatStreamEvent::ReasoningChunk(_) => {
							if !otel.first_chunk_recorded {
								otel.first_chunk_recorded = true;
								let seconds = otel.start.elapsed().as_secs_f64();
								crate::otel::span::record_time_to_first_chunk(&otel.span, seconds);
							}
						}
						ChatStreamEvent::End(stream_end) => {
							crate::otel::span::record_stream_end(&otel.span, stream_end);
						}
						_ => {}
					}
				}

				Poll::Ready(Some(Ok(chat_event)))
			}
			Poll::Ready(Some(Err(e))) => {
				// -- Frame sink: a mid-stream failure ends the stream without an End event, so this
				//    is the sink's only terminal notification. Streamers keep polling after most
				//    errors, so a stream can reach here more than once, and can still reach its End
				//    afterwards; the tap latches to keep it a single callback.
				if let Some(frame_tap) = this.frame_tap.as_ref() {
					frame_tap.on_error(&e);
				}

				#[cfg(feature = "otel")]
				if let Some(otel) = this.otel.as_ref() {
					crate::otel::span::record_error(&otel.span, &e);
				}
				Poll::Ready(Some(Err(e)))
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

// endregion: --- Stream Impl

// region:    --- ChatStreamEvent

/// Provider-agnostic chat events returned by `Client::exec()` when streaming.
#[derive(Debug, Serialize, Deserialize)]
pub enum ChatStreamEvent {
	/// Emitted once at the start of the stream.
	Start,

	/// Assistant content chunk (text).
	Chunk(StreamChunk),

	/// Reasoning content chunk.
	ReasoningChunk(StreamChunk),

	/// Thought signature content chunk.
	ThoughtSignatureChunk(StreamChunk),

	/// Tool-call chunk.
	ToolCallChunk(ToolChunk),

	/// End of stream.
	/// May include captured usage and/or content when enabled via `ChatOptions`.
	End(StreamEnd),

	/// Ping emitted periodically to indicate the stream is still active.
	Heartbeat,
}

/// Content of `ChatStreamEvent::Chunk`.
/// Currently text only.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamChunk {
	/// Text content.
	pub content: String,
}

/// Content of `ChatStreamEvent::ToolCallChunk`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChunk {
	/// The tool call.
	pub tool_call: ToolCall,
}

/// Terminal event data with optionally captured usage and content.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StreamEnd {
	/// Captured usage if `ChatOptions.capture_usage` is enabled.
	pub captured_usage: Option<Usage>,

	/// Normalised stop reason captured at stream end (see [`StopReason`]).
	pub captured_stop_reason: Option<StopReason>,

	/// Captured final content (text and tool calls) if `ChatOptions.capture_content`
	/// or `capture_tool_calls` is enabled.
	/// Note: Since 0.4.0 this includes tool calls as well (for API symmetry with `ChatResponse`);
	///       use `.captured_tool_calls()` or `.captured_texts()`.
	pub captured_content: Option<MessageContent>,

	/// Captured reasoning content if `ChatOptions.capture_reasoning` is enabled.
	pub captured_reasoning_content: Option<String>,

	/// Response ID for stateful sessions (OpenAI Responses API).
	pub captured_response_id: Option<String>,
}

impl From<InterStreamEnd> for StreamEnd {
	fn from(inter_end: InterStreamEnd) -> Self {
		let captured_text_content = inter_end.captured_text_content;
		let mut captured_tool_calls = inter_end.captured_tool_calls;

		// -- create public captured_content
		// Ordering policy: paired ThoughtSignature/ReasoningContent blocks, then
		// unpaired signatures, text, and tool calls. Provider adapters can scan the
		// leading parts to reconstruct their native continuation blocks.
		let mut leading_content: Vec<ContentPart> = Vec::new();
		if let Some(captured_blocks) = inter_end.captured_thought_blocks {
			for block in captured_blocks {
				leading_content.push(ContentPart::ThoughtSignature(block.signature));
				if let Some(reasoning_content) = block.reasoning_content {
					leading_content.push(ContentPart::ReasoningContent(reasoning_content));
				}
			}
		}
		if let Some(captured_thoughts) = inter_end.captured_thought_signatures {
			leading_content.extend(captured_thoughts.into_iter().map(ContentPart::ThoughtSignature));
		}

		let mirrored_signatures = leading_content
			.iter()
			.filter_map(|part| part.as_thought_signature().map(str::to_string))
			.collect::<Vec<_>>();
		if !mirrored_signatures.is_empty() {
			// Also attach thoughts to the first tool call so that
			// ChatMessage::from(Vec<ToolCall>) can auto-prepend them.
			if let Some(tool_calls) = captured_tool_calls.as_mut()
				&& let Some(first_call) = tool_calls.first_mut()
			{
				first_call.thought_signatures = Some(mirrored_signatures);
			}
		}

		let mut captured_content = (!leading_content.is_empty()).then(|| MessageContent::from_parts(leading_content));
		if let Some(captured_text_content) = captured_text_content {
			// This `captured_text_content` is the concatenation of all text chunks received.
			if let Some(existing_content) = &mut captured_content {
				existing_content.extend(MessageContent::from_text(captured_text_content));
			} else {
				captured_content = Some(MessageContent::from_text(captured_text_content));
			}
		}
		if let Some(captured_tool_calls) = captured_tool_calls {
			if let Some(existing_content) = &mut captured_content {
				existing_content.extend(MessageContent::from_tool_calls(captured_tool_calls));
			} else {
				// This `captured_tool_calls` is the concatenation of all tool call chunks received.
				captured_content = Some(MessageContent::from_tool_calls(captured_tool_calls));
			}
		}

		// -- Return result
		StreamEnd {
			captured_usage: inter_end.captured_usage,
			captured_stop_reason: inter_end.captured_stop_reason,
			captured_content,
			captured_reasoning_content: inter_end.captured_reasoning_content,
			captured_response_id: inter_end.captured_response_id,
		}
	}
}

/// Getters
impl StreamEnd {
	/// Returns the first captured text, if any.
	/// This is the concatenation of all streamed text chunks.
	pub fn captured_first_text(&self) -> Option<&str> {
		let captured_content = self.captured_content.as_ref()?;
		captured_content.first_text()
	}

	/// Consumes `self` and returns the first captured text, if any.
	/// This is the concatenation of all streamed text chunks.
	pub fn captured_into_first_text(self) -> Option<String> {
		let captured_content = self.captured_content?;
		captured_content.into_first_text()
	}

	/// Returns all captured text segments, if any.
	pub fn captured_texts(&self) -> Option<Vec<&str>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(captured_content.texts())
	}

	/// Consumes `self` and returns all captured text segments, if any.
	pub fn into_texts(self) -> Option<Vec<String>> {
		let captured_content = self.captured_content?;
		Some(captured_content.into_texts())
	}

	/// Returns all captured tool calls, if any.
	pub fn captured_tool_calls(&self) -> Option<Vec<&ToolCall>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(captured_content.tool_calls())
	}

	/// Consumes `self` and returns all captured tool calls, if any.
	pub fn captured_into_tool_calls(self) -> Option<Vec<ToolCall>> {
		let captured_content = self.captured_content?;
		Some(captured_content.into_tool_calls())
	}

	/// Returns all captured thought signatures, if any.
	pub fn captured_thought_signatures(&self) -> Option<Vec<&str>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(
			captured_content
				.parts()
				.iter()
				.filter_map(|p| p.as_thought_signature())
				.collect(),
		)
	}

	/// Consumes `self` and returns all captured thought signatures, if any.
	pub fn captured_into_thought_signatures(self) -> Option<Vec<String>> {
		let captured_content = self.captured_content?;
		Some(
			captured_content
				.into_parts()
				.into_iter()
				.filter_map(|p| p.into_thought_signature())
				.collect(),
		)
	}

	/// Convenience: build an assistant message for a tool-use handoff that places
	/// thought signatures (if any) before tool calls. Returns None if no tool calls
	/// were captured.
	pub fn into_assistant_message_for_tool_use(self) -> Option<ChatMessage> {
		let content = self.captured_content?;
		if content.tool_calls().is_empty() {
			return None;
		}
		let contains_reasoning = content.contains_reasoning_content();
		let mut message = ChatMessage::assistant(content);
		if !contains_reasoning {
			message = message.with_reasoning_content(self.captured_reasoning_content);
		}
		Some(message)
	}
}

// endregion: --- ChatStreamEvent

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ModelIden;
	use crate::adapter::AdapterKind;
	use crate::chat::{ChatFrameSink, FrameCtx, RawFrameRef};
	use futures::StreamExt;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[derive(Debug, Default)]
	struct TerminalSink {
		ends: AtomicUsize,
		errors: AtomicUsize,
	}

	impl ChatFrameSink for TerminalSink {
		fn on_frame(&self, _ctx: &FrameCtx, _frame: RawFrameRef<'_>) {}

		fn on_end(&self, _ctx: &FrameCtx, _end: &StreamEnd) {
			self.ends.fetch_add(1, Ordering::Relaxed);
		}

		fn on_error(&self, _ctx: &FrameCtx, _err: &crate::Error) {
			self.errors.fetch_add(1, Ordering::Relaxed);
		}
	}

	fn test_model() -> ModelIden {
		ModelIden::new(AdapterKind::OpenAI, "gpt-test")
	}

	fn chat_response_error() -> crate::Error {
		crate::Error::ChatResponse {
			model_iden: test_model(),
			body: serde_json::json!({"message": "boom"}),
		}
	}

	#[tokio::test]
	async fn test_chat_stream_fires_a_single_terminal_sink_callback() {
		// -- Setup & Fixtures
		// What an OpenAI-compatible provider produces for
		// `{"error":..}` / `{"error":..}` / `[DONE]`: the streamers do not set their `done`
		// flag on the error path, so polling continues and the stream still reaches its End.
		let sink = Arc::new(TerminalSink::default());
		let inter_stream = futures::stream::iter(vec![
			Ok(InterStreamEvent::Start),
			Err(chat_response_error()),
			Err(chat_response_error()),
			Ok(InterStreamEvent::End(InterStreamEnd::default())),
		]);
		let frame_tap = FrameTap::new(sink.clone(), FrameCtx::new(test_model()));
		let mut chat_stream = ChatStream::from_inter_stream(inter_stream).with_frame_tap(Some(frame_tap));

		// -- Exec
		// A caller that logs errors and keeps consuming (allowed by the `Stream` contract).
		let mut events = 0;
		while chat_stream.next().await.is_some() {
			events += 1;
		}

		// -- Check
		assert_eq!(events, 4, "the stream itself still yields every event");
		assert_eq!(sink.errors.load(Ordering::Relaxed), 1, "on_error should fire once");
		assert_eq!(
			sink.ends.load(Ordering::Relaxed),
			0,
			"on_end should not follow on_error for the same stream"
		);
	}

	#[test]
	fn test_stream_end_preserves_captured_stop_reason() {
		let inter_end = InterStreamEnd {
			captured_stop_reason: Some(StopReason::Completed("stop".to_string())),
			..Default::default()
		};
		let stream_end = StreamEnd::from(inter_end);
		assert_eq!(
			stream_end.captured_stop_reason,
			Some(StopReason::Completed("stop".to_string()))
		);
	}

	#[test]
	fn stream_end_orders_signature_text_and_tool_call_and_mirrors_signature() {
		let inter_end = InterStreamEnd {
			captured_thought_signatures: Some(vec!["opaque-signature".to_string()]),
			captured_text_content: Some("visible text".to_string()),
			captured_tool_calls: Some(vec![ToolCall {
				call_id: "call-1".to_string(),
				fn_name: "lookup".to_string(),
				fn_arguments: serde_json::json!({"query": "weather"}),
				thought_signatures: None,
			}]),
			..Default::default()
		};

		let end = StreamEnd::from(inter_end);
		let parts = end.captured_content.as_ref().expect("captured content").parts();
		assert!(matches!(&parts[0], ContentPart::ThoughtSignature(signature) if signature == "opaque-signature"));
		assert!(matches!(&parts[1], ContentPart::Text(text) if text == "visible text"));
		assert!(matches!(&parts[2], ContentPart::ToolCall(_)));

		let tool_call = end.captured_tool_calls().expect("tool calls")[0];
		assert_eq!(
			tool_call.thought_signatures.as_ref().expect("signature mirror"),
			&vec!["opaque-signature".to_string()]
		);
	}
}
