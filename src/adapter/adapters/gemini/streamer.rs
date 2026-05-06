use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::gemini::{GeminiAdapter, GeminiChatResponse};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, StopReason, ToolCall};
use crate::webc::{Event, EventSourceStream};
use crate::{Error, ModelIden, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::GeminiChatContent;

use std::collections::VecDeque;

pub struct GeminiStreamer {
	inner: EventSourceStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Once set, the next call to `poll_next` returns `Poll::Ready(None)` and stops polling
	/// the underlying SSE stream. Set right after the synthesized `InterStreamEvent::End` is yielded.
	done: bool,
	captured_data: StreamerCapturedData,
	pending_events: VecDeque<InterStreamEvent>,
}

impl GeminiStreamer {
	pub fn new(inner: EventSourceStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			pending_events: VecDeque::new(),
		}
	}

	/// Build the terminal `InterStreamEvent::End` from captured state and queue it.
	/// Drains all `captured_data` fields so subsequent calls return defaults.
	fn queue_end_event(&mut self) {
		let inter_stream_end = InterStreamEnd {
			captured_usage: self.captured_data.usage.take(),
			captured_stop_reason: self.captured_data.stop_reason.take().map(StopReason::from),
			captured_text_content: self.captured_data.content.take(),
			captured_reasoning_content: self.captured_data.reasoning_content.take(),
			captured_tool_calls: self.captured_data.tool_calls.take(),
			captured_thought_signatures: self.captured_data.thought_signatures.take(),
			captured_response_id: None,
		};
		self.pending_events.push_back(InterStreamEvent::End(inter_stream_end));
	}

	/// Pop the next pending event. If it is `End`, also flip `self.done` so we stop
	/// polling the upstream SSE on the next call.
	fn pop_pending(&mut self) -> Option<InterStreamEvent> {
		let event = self.pending_events.pop_front()?;
		if matches!(event, InterStreamEvent::End(_)) {
			self.done = true;
		}
		Some(event)
	}
}

impl futures::Stream for GeminiStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		// 1. Drain queued events first (content from prior chunk, or terminal End)
		if let Some(event) = self.pop_pending() {
			return Poll::Ready(Some(Ok(event)));
		}

		while let Poll::Ready(item) = Pin::new(&mut self.inner).poll_next(cx) {
			match item {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					// -- Parse the data payload as JSON (one Gemini chunk per SSE frame)
					let json_block = match serde_json::from_str::<Value>(&message.data) {
						Ok(v) => v,
						Err(serde_error) => {
							let err = Error::StreamParse {
								model_iden: self.options.model_iden.clone(),
								serde_error,
							};
							tracing::error!("Gemini Adapter Stream Error: {}", err);
							return Poll::Ready(Some(Err(err)));
						}
					};

					// -- Decode the chunk into the shared GeminiChatResponse shape
					let gemini_response =
						match GeminiAdapter::body_to_gemini_chat_response(&self.options.model_iden, json_block) {
							Ok(r) => r,
							Err(err) => {
								tracing::error!("Gemini Adapter Stream Error: {}", err);
								return Poll::Ready(Some(Err(err)));
							}
						};

					let GeminiChatResponse {
						content,
						usage,
						stop_reason,
					} = gemini_response;

					// -- Capture stop_reason if present (typically in the last chunk)
					let is_final_chunk = stop_reason.is_some();
					if stop_reason.is_some() {
						self.captured_data.stop_reason = stop_reason;
					}

					// -- Extract text, reasoning, and tool calls from content parts.
					// Gemini can return multiple functionCall parts in a single
					// streaming chunk (parallel tool calls), so we collect all of them.
					let mut stream_text_content: String = String::new();
					let mut stream_reasoning_content: Option<String> = None;
					let mut stream_tool_calls: Vec<ToolCall> = Vec::new();
					let mut stream_thought: Option<String> = None;

					for g_content_item in content {
						match g_content_item {
							GeminiChatContent::Reasoning(reasoning) => stream_reasoning_content = Some(reasoning),
							GeminiChatContent::Text(text) => stream_text_content.push_str(&text),
							GeminiChatContent::Binary(_) => {
								// For now, we do not stream binary content.
							}
							GeminiChatContent::ToolCall(tool_call) => stream_tool_calls.push(tool_call),
							GeminiChatContent::ThoughtSignature(thought) => stream_thought = Some(thought),
						}
					}

					// -- Always capture usage from the chunk if requested. Gemini sends usage on
					// every chunk; the last value (usually on the finishReason chunk) wins.
					if self.options.capture_usage {
						self.captured_data.usage = Some(usage);
					}

					// -- Queue Events. Priority: Thought -> Reasoning -> Text -> ToolCall

					if let Some(thought) = stream_thought {
						match self.captured_data.thought_signatures {
							Some(ref mut thoughts) => thoughts.push(thought.clone()),
							None => self.captured_data.thought_signatures = Some(vec![thought.clone()]),
						}
						self.pending_events.push_back(InterStreamEvent::ThoughtSignatureChunk(thought));
					}

					if let Some(reasoning_content) = stream_reasoning_content {
						if self.options.capture_content {
							match self.captured_data.reasoning_content {
								Some(ref mut rc) => rc.push_str(&reasoning_content),
								None => self.captured_data.reasoning_content = Some(reasoning_content.clone()),
							}
						}
						self.pending_events
							.push_back(InterStreamEvent::ReasoningChunk(reasoning_content));
					}

					if !stream_text_content.is_empty() {
						if self.options.capture_content {
							match self.captured_data.content {
								Some(ref mut c) => c.push_str(&stream_text_content),
								None => self.captured_data.content = Some(stream_text_content.clone()),
							}
						}
						self.pending_events.push_back(InterStreamEvent::Chunk(stream_text_content));
					}

					for tool_call in stream_tool_calls {
						if self.options.capture_tool_calls {
							match self.captured_data.tool_calls {
								Some(ref mut tool_calls) => tool_calls.push(tool_call.clone()),
								None => self.captured_data.tool_calls = Some(vec![tool_call.clone()]),
							}
						}
						self.pending_events.push_back(InterStreamEvent::ToolCallChunk(tool_call));
					}

					// -- If this chunk carried a finishReason, queue the terminal End right after
					// the chunk's own events. Gemini SSE has no separate [DONE] sentinel, so the
					// finishReason marks end-of-stream.
					if is_final_chunk {
						self.queue_end_event();
					}

					if let Some(event) = self.pop_pending() {
						return Poll::Ready(Some(Ok(event)));
					}
				}
				Some(Err(err)) => {
					tracing::error!("Gemini Adapter Stream Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}
				None => {
					// Defensive fallback: stream closed without a finishReason chunk. Still
					// emit a terminal End so downstream consumers can finalize.
					self.queue_end_event();
					if let Some(event) = self.pop_pending() {
						return Poll::Ready(Some(Ok(event)));
					}
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}
