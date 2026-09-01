use super::ix_types::{IxInteraction, ix_status_to_stop_reason};
use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions, new_frame_tap};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, ToolCall, Usage};
use crate::webc::{Event, EventSourceStream};
use crate::{Error, ModelIden, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};

/// SSE streamer for the Gemini Interactions API.
///
/// DOC: <https://ai.google.dev/gemini-api/docs/interactions/streaming>
pub struct GeminiIxStreamer {
	inner: EventSourceStream,
	options: StreamerOptions,
	done: bool,
	captured_data: StreamerCapturedData,
	interaction_id: Option<String>,
	in_progress_tool_calls: BTreeMap<usize, (ToolCall, String)>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "event_type")]
enum IxStreamEvent {
	#[serde(rename = "interaction.created")]
	InteractionCreated { interaction: IxInteraction },

	#[serde(rename = "interaction.status_update")]
	StatusUpdate {
		#[serde(default)]
		status: Option<String>,
	},

	#[serde(rename = "step.start")]
	StepStart {
		#[serde(default)]
		index: usize,
		step: Value,
	},

	#[serde(rename = "step.delta")]
	StepDelta {
		#[serde(default)]
		index: usize,
		delta: IxDelta,
	},

	#[serde(rename = "step.stop")]
	StepStop {
		#[serde(default)]
		index: usize,
	},

	#[serde(rename = "interaction.completed")]
	InteractionCompleted { interaction: IxInteraction },

	#[serde(rename = "error")]
	Error {
		#[serde(default)]
		error: Value,
	},

	#[serde(other)]
	Unknown,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IxDelta {
	Text {
		#[serde(default)]
		text: String,
	},

	ThoughtSummary {
		#[serde(default)]
		content: Value,
	},

	ThoughtSignature {
		#[serde(default)]
		signature: String,
	},

	ArgumentsDelta {
		#[serde(default)]
		arguments: String,
	},

	#[serde(other)]
	Other,
}

impl GeminiIxStreamer {
	pub fn new(inner: EventSourceStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		let frame_tap = new_frame_tap(&model_iden, &options_set);

		Self {
			inner: inner.with_frame_tap(frame_tap),
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			interaction_id: None,
			in_progress_tool_calls: BTreeMap::new(),
		}
	}

	/// Clones the frame tap (if any), so `ChatStream` can fire the terminal sink hooks.
	pub fn frame_tap(&self) -> Option<crate::webc::FrameTap> {
		self.inner.frame_tap()
	}

	fn build_stream_end(&mut self, response_id: Option<String>) -> InterStreamEnd {
		InterStreamEnd {
			captured_usage: self.captured_data.usage.take(),
			captured_stop_reason: ix_status_to_stop_reason(self.captured_data.stop_reason.take()),
			captured_text_content: self.captured_data.content.take(),
			captured_reasoning_content: self.captured_data.reasoning_content.take(),
			captured_tool_calls: self.captured_data.tool_calls.take(),
			captured_thought_signatures: self.captured_data.thought_signatures.take(),
			captured_thought_blocks: None,
			captured_response_id: response_id.or_else(|| self.interaction_id.clone()),
		}
	}
}

impl futures::Stream for GeminiIxStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),

				Some(Ok(Event::Message(message))) => {
					// The stream is terminated by `event: done` / `data: [DONE]`. By then the
					// `interaction.completed` event has already set `done`, so this is only
					// reached when the terminal event was missing — let the `None` branch below
					// synthesize the end.
					if message.data.trim() == "[DONE]" {
						continue;
					}

					let stream_event: IxStreamEvent = match serde_json::from_str(&message.data) {
						Ok(stream_event) => stream_event,
						Err(serde_error) => {
							tracing::warn!(
								"GeminiIxStreamer - fail to parse event (skipping). Cause: {serde_error}. Data: {}",
								message.data
							);
							continue;
						}
					};

					match stream_event {
						IxStreamEvent::InteractionCreated { interaction } => {
							self.interaction_id = interaction.id;
							continue;
						}

						IxStreamEvent::StatusUpdate { status } => {
							if let Some(status) = status {
								self.captured_data.stop_reason = Some(status);
							}
							return Poll::Ready(Some(Ok(InterStreamEvent::Heartbeat)));
						}

						IxStreamEvent::StepStart { index, step } => {
							// Only `function_call` steps need state: the name and id arrive here,
							// while the arguments stream in as `arguments_delta`.
							if step.get("type").and_then(Value::as_str) == Some("function_call") {
								let call_id = step.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
								let fn_name = step.get("name").and_then(Value::as_str).unwrap_or_default().to_string();

								let tool_call = ToolCall {
									call_id,
									fn_name,
									fn_arguments: Value::Null,
									thought_signatures: None,
								};
								self.in_progress_tool_calls.insert(index, (tool_call, String::new()));
							}
							continue;
						}

						IxStreamEvent::StepDelta { index, delta } => match delta {
							IxDelta::Text { text } => {
								if self.options.capture_content {
									match self.captured_data.content {
										Some(ref mut content) => content.push_str(&text),
										None => self.captured_data.content = Some(text.clone()),
									}
								}
								return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(text))));
							}

							IxDelta::ThoughtSummary { content } => {
								// `{"type":"thought_summary","content":{"type":"text","text":".."}}`
								let Some(text) = content.get("text").and_then(Value::as_str) else {
									continue;
								};
								let text = text.to_string();
								if self.options.capture_reasoning_content {
									match self.captured_data.reasoning_content {
										Some(ref mut reasoning) => reasoning.push_str(&text),
										None => self.captured_data.reasoning_content = Some(text.clone()),
									}
								}
								return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(text))));
							}

							IxDelta::ThoughtSignature { signature } => {
								if signature.is_empty() {
									continue;
								}
								self.captured_data
									.thought_signatures
									.get_or_insert_with(Vec::new)
									.push(signature.clone());
								return Poll::Ready(Some(Ok(InterStreamEvent::ThoughtSignatureChunk(signature))));
							}

							IxDelta::ArgumentsDelta { arguments } => {
								if let Some((_, args_buffer)) = self.in_progress_tool_calls.get_mut(&index) {
									args_buffer.push_str(&arguments);
								}
								continue;
							}

							IxDelta::Other => continue,
						},

						IxStreamEvent::StepStop { index } => {
							// A finished `function_call` step is the only place a complete tool call
							// can be assembled — `interaction.completed` carries no steps.
							let Some((mut tool_call, args_buffer)) = self.in_progress_tool_calls.remove(&index) else {
								continue;
							};

							tool_call.fn_arguments = if args_buffer.trim().is_empty() {
								Value::Object(Default::default())
							} else {
								serde_json::from_str(&args_buffer).unwrap_or_else(|serde_error| {
									tracing::warn!(
										"GeminiIxStreamer - fail to parse tool call arguments for '{}'. \
										 Cause: {serde_error}. Passing through as a string.",
										tool_call.fn_name
									);
									Value::String(args_buffer.clone())
								})
							};

							if self.options.capture_tool_calls {
								let is_first = self.captured_data.tool_calls.as_ref().is_none_or(Vec::is_empty);
								let signatures =
									self.captured_data.thought_signatures.clone().filter(|s| !s.is_empty());
								if is_first && let Some(signatures) = signatures {
									tool_call.thought_signatures = Some(signatures);
								}
								self.captured_data
									.tool_calls
									.get_or_insert_with(Vec::new)
									.push(tool_call.clone());
							}

							return Poll::Ready(Some(Ok(InterStreamEvent::ToolCallChunk(tool_call))));
						}

						IxStreamEvent::InteractionCompleted { interaction } => {
							self.done = true;

							if self.options.capture_usage {
								self.captured_data.usage = interaction.usage.map(Usage::from);
							}
							if let Some(status) = interaction.status {
								self.captured_data.stop_reason = Some(status);
							}

							let inter_stream_end = self.build_stream_end(interaction.id);
							return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
						}

						IxStreamEvent::Error { error } => {
							self.done = true;
							let message = error
								.get("message")
								.and_then(Value::as_str)
								.unwrap_or("Gemini Interactions stream error");

							return Poll::Ready(Some(Err(Error::StreamParse {
								model_iden: self.options.model_iden.clone(),
								serde_error: serde::de::Error::custom(message),
							})));
						}

						IxStreamEvent::Unknown => continue,
					}
				}

				Some(Err(err)) => {
					tracing::error!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}

				None => {
					if !self.done {
						self.done = true;
						let inter_stream_end = self.build_stream_end(None);
						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}
					return Poll::Ready(None);
				}
			}
		}

		Poll::Pending
	}
}

// region:    --- Tests

#[cfg(test)]
#[path = "streamer_tests.rs"]
mod tests;

// endregion: --- Tests
