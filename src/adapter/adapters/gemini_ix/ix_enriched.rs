//! Reading provider specific details out of raw stream frames.
//!
//! The normalized `ChatStreamEvent` surface models what every provider has in common. The
//! Interactions API emits more than that, and some of it has no genai equivalent at all:
//!
//! - **Server-side tools** (Google Search, code execution, URL context) run inside the provider.
//!   They never become a `ToolCall` and are charged
//! - **Citations.** `ContentPart::Text` is a plain `String`; there is nowhere to attach the
//!   `url_citation` annotations. Gemini API mandates that we need to cite sources
//!    as per their terms-of-service
//! - **Grounding billing counts**, Usage does not have fields for server tool calls
//!
//!
//! ```no_run
//! use genai::adapter::gemini_interactions::EnrichedEvent;
//! use genai::chat::{ChatFrameSink, FrameCtx, RawFrameRef};
//!
//! struct Sink;
//! impl ChatFrameSink for Sink {
//!     fn on_frame(&self, _ctx: &FrameCtx, frame: RawFrameRef<'_>) {
//!         match EnrichedEvent::from_frame(&frame) {
//!             Some(EnrichedEvent::SearchCall { queries }) => println!("searched: {queries:?}"),
//!             Some(EnrichedEvent::UrlCitations(citations)) => println!("{} citations", citations.len()),
//!             _ => (),
//!         }
//!     }
//! }
//! ```

use crate::chat::RawFrameRef;
use serde_json::Value;

/// Source attribution for one byte range of the answer.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:TextContent> (`annotations`)
#[derive(Debug, Clone, PartialEq)]
pub struct UrlCitation {
	/// Start of the attributed span, as a **byte** offset into the answer text.
	pub start_index: usize,

	/// End of the attributed span (exclusive), as a **byte** offset.
	///
	/// NOTE: The span typically ends at the last content byte, i.e. *before* the sentence's
	/// closing punctuation.
	pub end_index: usize,

	/// The source URL. For Google Search grounding this is a `vertexaisearch.cloud.google.com`
	/// redirect, not the origin URL.
	pub url: String,

	/// Display title, usually the source domain.
	pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroundingToolCount {
	/// Tool identifier, e.g. `google_search`.
	pub tool: String,
	/// Number of grounding invocations.
	pub count: u64,
	/// Number of distinct search queries issued, when the tool reports it.
	pub search_query_count: Option<u64>,
}

/// Provider detail decoded from one raw stream frame.
///
/// Built by [`EnrichedEvent::from_frame`]. Variants cover what has been verified against the live
/// API; anything else server-side arrives as [`EnrichedEvent::ServerTool`] with its payload
/// intact, so a protocol addition surfaces rather than disappearing.
#[derive(Debug, Clone, PartialEq)]
pub enum EnrichedEvent {
	/// A step began. `step_type` is the wire tag: `thought`, `model_output`, `function_call`,
	/// `google_search_call`, …
	StepStart { index: u64, step_type: String },

	/// A step finished. Pairs with [`EnrichedEvent::StepStart`] by `index`.
	StepStop { index: u64 },

	/// Google Search ran, with the queries the model chose.
	SearchCall { queries: Vec<String> },

	/// Google Search returned.
	SearchResult {
		is_error: bool,
		/// Google's rendered "search suggestion" chips. Their terms require displaying these
		/// alongside a grounded answer. Large — tens of KB per call.
		search_suggestions: Option<String>,
	},

	/// Source attribution for the answer produced so far.
	UrlCitations(Vec<UrlCitation>),

	/// Grounding invocation counts, from the terminal `interaction.completed` frame.
	GroundingCounts(Vec<GroundingToolCount>),

	/// Any other server-side tool call or result (`code_execution_*`, `url_context_*`,
	/// `file_search_*`, `google_maps_*`, `mcp_server_tool_*`), with the raw delta payload.
	ServerTool { kind: String, payload: Value },
}

impl EnrichedEvent {
	/// Decodes one raw frame, or `None` when it carries nothing the normalized stream lacks.
	///
	/// Cheap to call on every frame: the SSE event name is checked before any JSON is parsed, so
	/// unrelated frames cost a string comparison.
	pub fn from_frame(frame: &RawFrameRef<'_>) -> Option<Self> {
		// `event` is the SSE event name, present on every Interactions frame.
		match frame.event? {
			"step.start" => {
				let payload = frame.json()?;
				Some(Self::StepStart {
					index: payload.get("index").and_then(Value::as_u64).unwrap_or(0),
					step_type: payload.pointer("/step/type").and_then(Value::as_str)?.to_string(),
				})
			}

			"step.stop" => {
				let payload = frame.json()?;
				Some(Self::StepStop {
					index: payload.get("index").and_then(Value::as_u64).unwrap_or(0),
				})
			}

			"step.delta" => {
				let payload = frame.json()?;
				let delta = payload.get("delta")?;
				Self::from_delta(delta)
			}

			"interaction.completed" => {
				let payload = frame.json()?;
				let counts = payload
					.pointer("/interaction/usage/grounding_tool_count")
					.and_then(Value::as_array)?;
				let counts: Vec<GroundingToolCount> = counts
					.iter()
					.filter_map(|entry| {
						Some(GroundingToolCount {
							tool: entry.get("type").and_then(Value::as_str)?.to_string(),
							count: entry.get("count").and_then(Value::as_u64).unwrap_or(0),
							search_query_count: entry.get("search_query_count").and_then(Value::as_u64),
						})
					})
					.collect();
				(!counts.is_empty()).then_some(Self::GroundingCounts(counts))
			}

			_ => None,
		}
	}

	/// The `step.delta` half, split out to keep `from_frame` readable.
	fn from_delta(delta: &Value) -> Option<Self> {
		let delta_type = delta.get("type").and_then(Value::as_str)?;

		match delta_type {
			"google_search_call" => {
				let queries = delta
					.pointer("/arguments/queries")
					.and_then(Value::as_array)
					.map(|queries| queries.iter().filter_map(Value::as_str).map(String::from).collect())
					.unwrap_or_default();
				Some(Self::SearchCall { queries })
			}

			"google_search_result" => Some(Self::SearchResult {
				is_error: delta.get("is_error").and_then(Value::as_bool).unwrap_or(false),
				// `result` is an array; the suggestions ride on the first entry.
				search_suggestions: delta
					.pointer("/result/0/search_suggestions")
					.and_then(Value::as_str)
					.map(String::from),
			}),

			"text_annotation_delta" => {
				let citations: Vec<UrlCitation> = delta
					.get("annotations")
					.and_then(Value::as_array)?
					.iter()
					.filter(|annotation| annotation.get("type").and_then(Value::as_str) == Some("url_citation"))
					.filter_map(|annotation| {
						Some(UrlCitation {
							start_index: annotation.get("start_index").and_then(Value::as_u64)? as usize,
							end_index: annotation.get("end_index").and_then(Value::as_u64)? as usize,
							url: annotation.get("url").and_then(Value::as_str)?.to_string(),
							title: annotation.get("title").and_then(Value::as_str).map(String::from),
						})
					})
					.collect();
				(!citations.is_empty()).then_some(Self::UrlCitations(citations))
			}

			// Everything else server-side. `text`, `thought_summary`, `thought_signature` and
			// `arguments_delta` are all normalized already, so they are deliberately not here.
			other if other.ends_with("_call") || other.ends_with("_result") => Some(Self::ServerTool {
				kind: other.to_string(),
				payload: delta.clone(),
			}),

			_ => None,
		}
	}
}

// region:    --- Tests

#[cfg(test)]
#[path = "ix_enriched_tests.rs"]
mod tests;

// endregion: --- Tests
