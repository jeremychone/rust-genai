//! Use `ChatFrameSSink` to capture the raw frames and decode them with `EnrichedEvent::from_frame`.
//!
//! `EnrichedEvent::from_frame` (in `genai::adapter::gemini_interactions`) decodes a raw frame ,
//! so a client does not have to hand-write JSON pointer walks.
//!
//! A `ChatFrameSink` sees every frame as it decodes, so this example runs in two phases. The
//! answer streams token by token while the sink reads the same frames for grounding detail;
//! then, once the whole text exists for the byte offsets to
//! index into, it is re-rendered with numbered citation markers and a source list. Citations only
//! arrive near the end of the stream, so that second pass is unavoidable — it is the pattern you
//! would actually ship.
//!
//! Requires: GEMINI_API_KEY environment variable.
//!
//! Run: `GEMINI_API_KEY=... cargo run --example c99-gemini-search-citations`

use futures::StreamExt;
use genai::adapter::gemini_interactions::{EnrichedEvent, GroundingToolCount, UrlCitation};
use genai::chat::{ChatFrameSink, ChatOptions, ChatRequest, ChatStreamEvent, FrameCtx, RawFrameRef, Tool, ToolName};
use genai::{Client, Error};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::AsyncWriteExt;

const MODEL: &str = "gemini_ix::gemini-3.5-flash-lite";

/// Collects everything the grounding machinery emits that the normalized types drop.
///
/// Bounded memory: queries and citations only. The `google_search_result` payload is large
/// (tens of KB of rendered HTML per call) and is counted, not retained.
#[derive(Debug, Default)]
struct Grounding {
	queries: Mutex<Vec<String>>,
	citations: Mutex<Vec<UrlCitation>>,
	grounding_counts: Mutex<Vec<GroundingToolCount>>,
	searches: AtomicUsize,
	result_bytes: AtomicUsize,
}

impl ChatFrameSink for Grounding {
	fn on_frame(&self, _ctx: &FrameCtx, frame: RawFrameRef<'_>) {
		// One call decodes the frame. The JSON pointer walks, the delta-type tags and the
		// citation shape all live in the adapter, where the protocol knowledge belongs.
		match EnrichedEvent::from_frame(&frame) {
			// The queries the model chose to run — never a `ToolCall`, because search is
			// server-side.
			Some(EnrichedEvent::SearchCall { queries }) => {
				self.searches.fetch_add(1, Ordering::Relaxed);
				self.queries.lock().unwrap().extend(queries);
			}

			// Google's rendered suggestion chips, which their terms require displaying next to a
			// grounded answer. Tens of KB per call, so this records the size; a real client would
			// keep the HTML.
			Some(EnrichedEvent::SearchResult { search_suggestions, .. }) => {
				let len = search_suggestions.map(|html| html.len()).unwrap_or(0);
				self.result_bytes.fetch_add(len, Ordering::Relaxed);
			}

			// The attribution itself: byte ranges of the answer mapped to source URLs.
			Some(EnrichedEvent::UrlCitations(citations)) => {
				self.citations.lock().unwrap().extend(citations);
			}

			// Billing counts for the grounding tools, which `Usage` has no field for.
			Some(EnrichedEvent::GroundingCounts(counts)) => {
				*self.grounding_counts.lock().unwrap() = counts;
			}

			_ => (),
		}
	}

	fn on_error(&self, _ctx: &FrameCtx, err: &Error) {
		println!("\n-- sink: stream failed: {err}");
	}
}

/// Renders the answer with `[n]` markers inserted at the end of each cited span, followed by the
/// numbered source list.
///
/// `start_index` / `end_index` are **byte** offsets into the answer text, and several sources
/// commonly attribute the same span, so spans are grouped and sources numbered by first use.
fn render_with_citations(answer: &str, citations: &[UrlCitation]) -> String {
	// -- Number the sources by first appearance, deduplicated by URL.
	let mut source_order: Vec<(&str, &str)> = Vec::new(); // (url, title)
	for citation in citations {
		if !source_order.iter().any(|(url, _)| *url == citation.url) {
			source_order.push((&citation.url, citation.title.as_deref().unwrap_or("untitled")));
		}
	}

	// -- Group source numbers by the span they cite.
	let mut by_span: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
	for citation in citations {
		let number = source_order.iter().position(|(url, _)| *url == citation.url).unwrap_or(0) + 1;
		let markers = by_span.entry(citation.end_index.min(answer.len())).or_default();
		if !markers.contains(&number) {
			markers.push(number);
		}
	}

	// -- Splice the markers in, walking byte offsets and refusing to split a character.
	// The marker lands exactly on the annotated boundary, which typically sits just *before* the
	// sentence's closing punctuation ("…constraints[1]." rather than "…constraints.[1]"). Moving
	// it past the punctuation is a presentation choice; this keeps to what the provider marked.
	let mut out = String::with_capacity(answer.len() + by_span.len() * 8);
	let mut cursor = 0usize;
	for (at, mut numbers) in by_span {
		if at < cursor || !answer.is_char_boundary(at) {
			continue;
		}
		numbers.sort_unstable();
		out.push_str(&answer[cursor..at]);
		for number in numbers {
			out.push_str(&format!("[{number}]"));
		}
		cursor = at;
	}
	out.push_str(&answer[cursor..]);

	// -- Source list
	out.push_str("\n\n=== Sources\n");
	for (index, (url, title)) in source_order.iter().enumerate() {
		out.push_str(&format!("  [{}] {title}\n      {url}\n", index + 1));
	}
	out
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "What were the headline features of the most recent Rust release? Two sentences.";

	let grounding = Arc::new(Grounding::default());
	let sink: Arc<dyn ChatFrameSink> = grounding.clone();
	// The sink is the only option needed: the answer comes back from `print_chat_stream`, and
	// everything else of interest is on the wire rather than in `ChatResponse`.
	let options = ChatOptions::default().with_raw_frame_sink_arc(sink);

	let client = Client::new()?;
	let chat_req = ChatRequest::from_user(question).append_tool(Tool::new(ToolName::WebSearch));

	// -- Stream it. Citations arrive near the end, so they cannot be rendered inline.
	println!("=== Question\n{question}\n\n=== Streaming\n");
	let chat_res = client.exec_chat_stream(MODEL, chat_req, Some(&options)).await?;
	let mut stream = chat_res.stream;

	// not using printer as it also prints the thought signature
	let mut stdout = tokio::io::stdout();
	let mut answer = String::new();

	while let Some(event) = stream.next().await {
		if let ChatStreamEvent::Chunk(chunk) = event? {
			stdout.write_all(chunk.content.as_bytes()).await?;
			stdout.flush().await?;
			answer.push_str(&chunk.content);
		}
	}
	stdout.write_all(b"\n").await?;
	stdout.flush().await?;

	let citations = grounding.citations.lock().unwrap().clone();
	let queries = grounding.queries.lock().unwrap().clone();

	println!("\n\n=== Answer, attributed");
	if citations.is_empty() {
		println!("\n{answer}\n\n(no citations returned — the model answered without grounding)");
	} else {
		println!("\n{}", render_with_citations(&answer, &citations));
	}

	println!(
		"=== Searches the model ran — {} queries across {} search call(s)",
		queries.len(),
		grounding.searches.load(Ordering::Relaxed)
	);
	for query in &queries {
		println!("  - {query}");
	}
	// How much of the answer is actually attributed? Uncited spans are the ones to be wary of.
	let mut spans: Vec<(usize, usize)> = citations.iter().map(|c| (c.start_index, c.end_index)).collect();
	spans.sort_unstable();
	spans.dedup();
	let covered: usize = spans.iter().map(|(start, end)| end.saturating_sub(*start)).sum();

	println!(
		"\n{} citation annotations over {} distinct span(s), covering {covered} of {} answer bytes. \
		 {} bytes of search-result payload (Google's suggestion chips) were seen and discarded.",
		citations.len(),
		spans.len(),
		answer.len(),
		grounding.result_bytes.load(Ordering::Relaxed),
	);

	for count in grounding.grounding_counts.lock().unwrap().iter() {
		println!(
			"Grounding billed: {} × {} ({} search queries)",
			count.count,
			count.tool,
			count.search_query_count.unwrap_or(0)
		);
	}

	Ok(())
}
