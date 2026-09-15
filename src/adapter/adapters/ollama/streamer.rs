use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions, new_frame_tap};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, StopReason, ToolCall, Usage};
use crate::error::BoxError;
use crate::webc::WebStream;
use crate::{Error, ModelIden, Result};
use serde_json::Value;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

/// Ollama streamer for `application/x-ndjson` response.
///
/// Ref: <https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion>
pub struct OllamaStreamer<S> {
	inner: S,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling after a done event
	done: bool,

	/// Events parsed from already-polled lines but not yet emitted. A single NDJSON line
	/// can carry several event-bearing fields (e.g., `thinking` and `content`), and
	/// `poll_next` can only return one item per call, so the remaining ones are buffered
	/// here and drained before polling the inner stream again.
	events: VecDeque<InterStreamEvent>,

	captured_data: StreamerCapturedData,
}

impl<S> OllamaStreamer<S> {
	fn new_with_inner(inner: S, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			events: VecDeque::new(),
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}

	/// Parses a single NDJSON line and queues **all** of its event-bearing fields, in
	/// stream order: reasoning, then text content, then tool calls (one event per tool
	/// call). Fields that share a line with another event are never silently dropped.
	fn process_line(&mut self, line: &str) -> Result<()> {
		let mut data: Value = match serde_json::from_str(line) {
			Ok(val) => val,
			Err(serde_error) => {
				return Err(Error::StreamParse {
					model_iden: self.options.model_iden.clone(),
					serde_error,
				});
			}
		};

		// -- Handle Reasoning Content Chunk
		// Ollama API doc mentions `thinking` field in message object.
		// Some models (like DeepSeek) might also use `reasoning_content`.
		let reasoning = data
			.x_take::<String>("/message/thinking")
			.or_else(|_| data.x_take::<String>("/message/reasoning_content"));

		if let Ok(reasoning) = reasoning {
			// Note: Ollama may return reasoning in chunks, so we check if it's non-empty before queuing it as a reasoning chunk.
			if !reasoning.is_empty() {
				// Add to the captured_reasoning_content if chat options say so
				if self.options.capture_reasoning_content {
					match self.captured_data.reasoning_content {
						Some(ref mut r) => r.push_str(&reasoning),
						None => self.captured_data.reasoning_content = Some(reasoning.clone()),
					}
				}
				self.events.push_back(InterStreamEvent::ReasoningChunk(reasoning));
			}
		}

		// -- Handle Text Chunk
		if let Ok(content) = data.x_take::<String>("/message/content") {
			// Note: Ollama may return content in chunks, so we check if it's non-empty before queuing it as a content chunk.
			if !content.is_empty() {
				// Add to the captured_content if chat options say so
				if self.options.capture_content {
					match self.captured_data.content {
						Some(ref mut c) => c.push_str(&content),
						None => self.captured_data.content = Some(content.clone()),
					}
				}
				self.events.push_back(InterStreamEvent::Chunk(content));
			}
		}

		// -- Handle Tool Calls Chunks
		// Every tool call of the line is emitted as its own ToolCallChunk event.
		if let Ok(tool_calls_value) = data.x_take::<Vec<Value>>("/message/tool_calls") {
			let mut tcs = Vec::new();
			for mut tc_val in tool_calls_value {
				let fn_name: String = tc_val.x_take("/function/name")?;
				let fn_arguments: Value = tc_val.x_take("/function/arguments")?;

				// GenAI requires a call_id.
				// Native Ollama API doesn't always provide one. Generate one if missing.
				let call_id = tc_val
					.x_take::<String>("/id")
					.unwrap_or_else(|_| format!("call_{}", &uuid::Uuid::new_v4().to_string()[..8]));

				tcs.push(ToolCall {
					call_id,
					fn_name,
					fn_arguments,
					thought_signatures: None,
				});
			}

			if !tcs.is_empty() {
				if self.options.capture_tool_calls {
					match self.captured_data.tool_calls {
						Some(ref mut existing) => existing.extend(tcs.iter().cloned()),
						None => self.captured_data.tool_calls = Some(tcs.clone()),
					}
				}
				for tc in tcs {
					self.events.push_back(InterStreamEvent::ToolCallChunk(tc));
				}
			}
		}

		// -- Handle Message Stop / Done
		let done = data.x_get::<bool>("/done").unwrap_or(false);
		if done {
			self.done = true;

			// Capture done_reason (e.g., "stop", "length")
			self.captured_data.stop_reason = data.x_take::<String>("done_reason").ok();

			if self.options.capture_usage {
				let prompt_tokens = data.x_get::<i32>("/prompt_eval_count").ok();
				let completion_tokens = data.x_get::<i32>("/eval_count").ok();
				let total_tokens = match (prompt_tokens, completion_tokens) {
					(Some(p), Some(c)) => Some(p + c),
					_ => None,
				};

				self.captured_data.usage = Some(Usage {
					prompt_tokens,
					completion_tokens,
					total_tokens,
					..Default::default()
				});
			}

			// The End event is queued (not returned immediately) so that any content,
			// reasoning, or tool-call events of the same line are emitted first.
			let inter_stream_end = self.build_inter_stream_end();
			self.events.push_back(InterStreamEvent::End(inter_stream_end));
		}

		Ok(())
	}

	/// Builds the final `InterStreamEnd` from the accumulated captures.
	fn build_inter_stream_end(&mut self) -> InterStreamEnd {
		InterStreamEnd {
			captured_usage: self.captured_data.usage.take(),
			captured_stop_reason: self.captured_data.stop_reason.take().map(StopReason::from),
			captured_text_content: self.captured_data.content.take(),
			captured_reasoning_content: self.captured_data.reasoning_content.take(),
			captured_tool_calls: self.captured_data.tool_calls.take(),
			captured_thought_signatures: None,
			captured_thought_blocks: None,
			captured_response_id: None,
		}
	}
}

impl OllamaStreamer<WebStream> {
	pub fn new(inner: WebStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		let frame_tap = new_frame_tap(&model_iden, &options_set);
		let mut streamer = Self::new_with_inner(inner, model_iden, options_set);
		streamer.inner = streamer.inner.with_frame_tap(frame_tap);
		streamer
	}

	/// Clones the frame tap (if any), so `ChatStream` can fire the terminal sink hooks.
	pub fn frame_tap(&self) -> Option<crate::webc::FrameTap> {
		self.inner.frame_tap()
	}
}

impl<S> futures::Stream for OllamaStreamer<S>
where
	S: futures::Stream<Item = std::result::Result<String, BoxError>> + Unpin,
{
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			// -- First, emit the events buffered from previously processed lines, so that
			//    every event-bearing field of a line is emitted (reasoning, then text,
			//    then tool calls) instead of dropping the ones after the first match.
			if let Some(event) = self.events.pop_front() {
				return Poll::Ready(Some(Ok(event)));
			}

			if self.done {
				return Poll::Ready(None);
			}

			match Pin::new(&mut self.inner).poll_next(cx) {
				Poll::Ready(Some(Ok(data_str))) => {
					// Ollama returns ndjson, so each line or chunk is a full JSON object.
					for line in data_str.lines() {
						if line.trim().is_empty() {
							continue;
						}

						self.process_line(line)?;

						// A done line ends the stream; the rest of the chunk is ignored
						// (the End event is queued and will be emitted last).
						if self.done {
							break;
						}
					}
					// Loop to drain the buffered events, or keep polling when the chunk
					// carried no event-bearing field.
				}
				Poll::Ready(Some(Err(err))) => {
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}
				Poll::Ready(None) => {
					// The inner stream ended without a done event; synthesize a final End.
					self.done = true;
					return Poll::Ready(Some(Ok(InterStreamEvent::End(self.build_inter_stream_end()))));
				}
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::AdapterKind;
	use crate::chat::ChatOptions;
	use futures::StreamExt;
	use serde_json::json;

	type LineItem = std::result::Result<String, BoxError>;
	type LineStream = futures::stream::Iter<std::vec::IntoIter<LineItem>>;

	// -- Support

	/// Builds a streamer over a fixed sequence of raw "web chunks" (as delivered by
	/// `WebStream`: one NDJSON line per item, or several lines joined by `\n`).
	fn support_streamer(chunks: Vec<String>, options_set: ChatOptionsSet<'_, '_>) -> OllamaStreamer<LineStream> {
		let items: Vec<LineItem> = chunks.into_iter().map(Ok).collect();
		OllamaStreamer::new_with_inner(
			futures::stream::iter(items),
			ModelIden::new(AdapterKind::Ollama, "qwen3"),
			options_set,
		)
	}

	/// Collects all events of the stream (erroring on the first stream error).
	async fn support_collect_events<S>(mut streamer: S) -> Result<Vec<InterStreamEvent>>
	where
		S: futures::Stream<Item = Result<InterStreamEvent>> + Unpin,
	{
		let mut events = Vec::new();
		while let Some(event) = streamer.next().await {
			events.push(event?);
		}
		Ok(events)
	}

	/// Compact, comparable form of an event sequence.
	fn support_event_labels(events: &[InterStreamEvent]) -> Vec<String> {
		events
			.iter()
			.map(|event| match event {
				InterStreamEvent::ReasoningChunk(reasoning) => format!("reasoning:{reasoning}"),
				InterStreamEvent::Chunk(content) => format!("chunk:{content}"),
				InterStreamEvent::ToolCallChunk(tc) => format!("tool:{}:{}", tc.call_id, tc.fn_name),
				InterStreamEvent::End(_) => "end".to_string(),
				other => format!("{other:?}"),
			})
			.collect()
	}

	// -- Tests

	/// Regression: a line carrying BOTH a non-empty `thinking` and a non-empty `content`
	/// must emit both events; previously the content was silently lost.
	#[tokio::test]
	async fn test_stream_line_with_thinking_and_content_emits_both() -> Result<()> {
		// -- Setup & Fixtures
		let chunks = vec![
			// Typical first empty message
			r#"{"message":{"role":"assistant","content":""}}"#.to_string(),
			// Transition chunk: tail of the reasoning + first answer token
			r#"{"message":{"thinking":"Let me analyze the request. ","content":"{\"findings\":"}}"#.to_string(),
			// Final token + done on the same line
			r#"{"message":{"content":"[]}"},"done":true,"done_reason":"stop"}"#.to_string(),
		];

		// -- Exec
		let events = support_collect_events(support_streamer(chunks, ChatOptionsSet::default())).await?;

		// -- Check
		assert_eq!(
			support_event_labels(&events),
			vec![
				"reasoning:Let me analyze the request. ",
				"chunk:{\"findings\":",
				"chunk:[]}",
				"end",
			]
		);
		// The done fields of the last line are captured even though the line also carried content.
		let InterStreamEvent::End(end) = &events[3] else {
			panic!("expected End as last event, got {:?}", events[3]);
		};
		assert!(end.captured_stop_reason.is_some(), "done_reason should be captured");

		Ok(())
	}

	/// Regression: a line with content AND a `tool_calls` array emits the content and
	/// every tool call (previously only the first tool call was emitted).
	#[tokio::test]
	async fn test_stream_line_with_content_and_tool_calls_emits_all() -> Result<()> {
		// -- Setup & Fixtures
		let chunks = vec![r#"{"message":{"content":"Checking the weather now.","tool_calls":[{"function":{"name":"get_weather","arguments":{"city":"Paris"}},"id":"call_a"},{"function":{"name":"get_time","arguments":{}},"id":"call_b"}]}}"#.to_string()];

		// -- Exec
		let events = support_collect_events(support_streamer(chunks, ChatOptionsSet::default())).await?;

		// -- Check
		assert_eq!(
			support_event_labels(&events),
			vec![
				"chunk:Checking the weather now.",
				"tool:call_a:get_weather",
				"tool:call_b:get_time",
				"end",
			]
		);
		let InterStreamEvent::ToolCallChunk(tc_a) = &events[1] else {
			panic!("expected ToolCallChunk, got {:?}", events[1]);
		};
		assert_eq!(tc_a.fn_arguments, json!({"city": "Paris"}));
		let InterStreamEvent::ToolCallChunk(tc_b) = &events[2] else {
			panic!("expected second ToolCallChunk, got {:?}", events[2]);
		};
		assert_eq!(tc_b.fn_arguments, json!({}));

		Ok(())
	}

	/// Single event-field lines (the common case) still yield exactly one event per line.
	#[tokio::test]
	async fn test_stream_single_field_lines_yield_one_event_each() -> Result<()> {
		// -- Setup & Fixtures
		let chunks = vec![
			r#"{"message":{"thinking":"thinking part"}}"#.to_string(),
			r#"{"message":{"content":"content part"}}"#.to_string(),
		];

		// -- Exec
		let events = support_collect_events(support_streamer(chunks, ChatOptionsSet::default())).await?;

		// -- Check
		assert_eq!(
			support_event_labels(&events),
			vec!["reasoning:thinking part", "chunk:content part", "end",]
		);

		Ok(())
	}

	/// All fields of a combined line reach the captures of the `StreamEnd` event.
	#[tokio::test]
	async fn test_stream_captures_all_fields_sharing_a_line() -> Result<()> {
		// -- Setup & Fixtures
		let options = ChatOptions::default()
			.with_capture_content(true)
			.with_capture_reasoning_content(true)
			.with_capture_tool_calls(true)
			.with_capture_usage(true);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

		// One final line carrying the last reasoning, the last content token,
		// two tool calls, and the done payload.
		let chunks = vec![r#"{"message":{"thinking":"final check. ","content":"Answer. ","tool_calls":[{"function":{"name":"get_weather","arguments":{"city":"Paris"}},"id":"call_a"},{"function":{"name":"get_time","arguments":{}},"id":"call_b"}]},"done":true,"done_reason":"stop","prompt_eval_count":10,"eval_count":20}"#.to_string()];

		// -- Exec
		let events = support_collect_events(support_streamer(chunks, options_set)).await?;

		// -- Check
		assert_eq!(
			support_event_labels(&events),
			vec![
				"reasoning:final check. ",
				"chunk:Answer. ",
				"tool:call_a:get_weather",
				"tool:call_b:get_time",
				"end",
			]
		);
		let InterStreamEvent::End(end) = &events[4] else {
			panic!("expected End as last event, got {:?}", events[4]);
		};
		assert_eq!(end.captured_reasoning_content.as_deref(), Some("final check. "));
		assert_eq!(end.captured_text_content.as_deref(), Some("Answer. "));
		let tool_calls = end.captured_tool_calls.as_ref().expect("tool calls should be captured");
		assert_eq!(tool_calls.len(), 2);
		assert_eq!(tool_calls[0].call_id, "call_a");
		assert_eq!(tool_calls[1].call_id, "call_b");
		let usage = end.captured_usage.as_ref().expect("usage should be captured");
		assert_eq!(usage.prompt_tokens, Some(10));
		assert_eq!(usage.completion_tokens, Some(20));
		assert_eq!(usage.total_tokens, Some(30));

		Ok(())
	}

	/// Several NDJSON lines delivered in one web chunk keep their event order.
	#[tokio::test]
	async fn test_stream_multiple_lines_in_one_chunk_emit_in_order() -> Result<()> {
		// -- Setup & Fixtures
		let chunks = vec![format!(
			"{}\n{}",
			r#"{"message":{"thinking":"reasoning"}}"#,
			r#"{"message":{"content":"answer"}}"#
		)];

		// -- Exec
		let events = support_collect_events(support_streamer(chunks, ChatOptionsSet::default())).await?;

		// -- Check
		assert_eq!(
			support_event_labels(&events),
			vec!["reasoning:reasoning", "chunk:answer", "end",]
		);

		Ok(())
	}

	/// Malformed lines still surface as `Error::StreamParse` (no panic).
	#[tokio::test]
	async fn test_stream_malformed_line_yields_stream_parse_error() -> Result<()> {
		// -- Setup & Fixtures
		let chunks = vec!["not-json".to_string()];
		let mut streamer = support_streamer(chunks, ChatOptionsSet::default());

		// -- Exec
		let event = streamer.next().await;

		// -- Check
		assert!(matches!(event, Some(Err(Error::StreamParse { .. }))), "got {event:?}");

		Ok(())
	}
}

// endregion: --- Tests
