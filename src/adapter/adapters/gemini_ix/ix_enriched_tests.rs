use super::{EnrichedEvent, UrlCitation};
use crate::chat::RawFrameRef;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

/// Frames below are verbatim from a live `google_search` stream captured 2026-08-31, with the
/// multi-KB `signature` and `search_suggestions` blobs abbreviated.
fn frame<'a>(event: &'a str, data: &'a str) -> RawFrameRef<'a> {
	RawFrameRef {
		index: 0,
		event: Some(event),
		data,
		elapsed_us: 0,
	}
}

#[test]
fn test_ix_enriched_search_call() -> Result<()> {
	// -- Setup & Fixtures
	let data = r#"{"index":0,"delta":{"signature":"EsgGCsUG…","type":"google_search_call",
		"arguments":{"queries":["Rust release history 2026","latest Rust release 2026"]}},
		"event_type":"step.delta"}"#;

	// -- Exec
	let event = EnrichedEvent::from_frame(&frame("step.delta", data));

	// -- Check
	let Some(EnrichedEvent::SearchCall { queries }) = event else {
		return Err(format!("expected a SearchCall, got {event:?}").into());
	};
	assert_eq!(queries, ["Rust release history 2026", "latest Rust release 2026"]);

	Ok(())
}

#[test]
fn test_ix_enriched_search_result() -> Result<()> {
	// -- Setup & Fixtures
	let data = r#"{"index":1,"delta":{"signature":"ErLnAgqu…","type":"google_search_result",
		"result":[{"search_suggestions":"<style>.container{}</style>"}],"is_error":false},
		"event_type":"step.delta"}"#;

	// -- Exec
	let event = EnrichedEvent::from_frame(&frame("step.delta", data));

	// -- Check
	let Some(EnrichedEvent::SearchResult {
		is_error,
		search_suggestions,
	}) = event
	else {
		return Err(format!("expected a SearchResult, got {event:?}").into());
	};
	assert!(!is_error);
	assert_eq!(search_suggestions.as_deref(), Some("<style>.container{}</style>"));

	Ok(())
}

#[test]
fn test_ix_enriched_url_citations() -> Result<()> {
	// -- Setup & Fixtures
	// Two sources attributing the same span — the common case, and why callers must dedupe.
	let data = r#"{"index":5,"delta":{"annotations":[
		{"start_index":0,"end_index":195,"url":"https://vertexaisearch…/a","title":"rust-lang.org","type":"url_citation"},
		{"start_index":0,"end_index":195,"url":"https://vertexaisearch…/b","title":"linuxcompatible.org","type":"url_citation"}
		],"type":"text_annotation_delta"},"event_type":"step.delta"}"#;

	// -- Exec
	let event = EnrichedEvent::from_frame(&frame("step.delta", data));

	// -- Check
	let Some(EnrichedEvent::UrlCitations(citations)) = event else {
		return Err(format!("expected UrlCitations, got {event:?}").into());
	};
	assert_eq!(citations.len(), 2);
	assert_eq!(
		citations[0],
		UrlCitation {
			start_index: 0,
			end_index: 195,
			url: "https://vertexaisearch…/a".to_string(),
			title: Some("rust-lang.org".to_string()),
		}
	);
	assert_eq!(citations[1].title.as_deref(), Some("linuxcompatible.org"));

	Ok(())
}

#[test]
fn test_ix_enriched_grounding_counts() -> Result<()> {
	// -- Setup & Fixtures
	let data = r#"{"interaction":{"id":"","status":"completed","usage":{
		"total_tokens":1194,"total_input_tokens":224,
		"grounding_tool_count":[{"type":"google_search","count":5,"search_query_count":5}]}},
		"event_type":"interaction.completed"}"#;

	// -- Exec
	let event = EnrichedEvent::from_frame(&frame("interaction.completed", data));

	// -- Check
	let Some(EnrichedEvent::GroundingCounts(counts)) = event else {
		return Err(format!("expected GroundingCounts, got {event:?}").into());
	};
	assert_eq!(counts.len(), 1);
	assert_eq!(counts[0].tool, "google_search");
	assert_eq!(counts[0].count, 5);
	assert_eq!(counts[0].search_query_count, Some(5));

	Ok(())
}

#[test]
fn test_ix_enriched_step_boundaries() -> Result<()> {
	// -- Setup & Fixtures
	let start = r#"{"index":0,"step":{"id":"call_318919","signature":"","type":"google_search_call"},"event_type":"step.start"}"#;
	let stop = r#"{"index":0,"event_type":"step.stop"}"#;

	// -- Exec & Check
	assert_eq!(
		EnrichedEvent::from_frame(&frame("step.start", start)),
		Some(EnrichedEvent::StepStart {
			index: 0,
			step_type: "google_search_call".to_string()
		})
	);
	assert_eq!(
		EnrichedEvent::from_frame(&frame("step.stop", stop)),
		Some(EnrichedEvent::StepStop { index: 0 })
	);

	Ok(())
}

#[test]
fn test_ix_enriched_returns_none_for_normalized_content() -> Result<()> {
	// -- Setup & Fixtures
	// These all reach the caller as ChatStreamEvent variants already, so the helper stays quiet
	// rather than duplicating them.
	let cases = [
		(
			"step.delta",
			r#"{"index":5,"delta":{"text":"Released on August 20","type":"text"},"event_type":"step.delta"}"#,
		),
		(
			"step.delta",
			r#"{"index":4,"delta":{"signature":"Et0YCtoY…","type":"thought_signature"},"event_type":"step.delta"}"#,
		),
		(
			"step.delta",
			r#"{"index":0,"delta":{"type":"arguments_delta","arguments":"{\"city\":\"Paris\"}"},"event_type":"step.delta"}"#,
		),
		(
			"interaction.created",
			r#"{"interaction":{"id":"","status":"in_progress"},"event_type":"interaction.created"}"#,
		),
		// A completed interaction with no grounding has nothing extra to report.
		(
			"interaction.completed",
			r#"{"interaction":{"id":"v1_x","status":"completed","usage":{"total_tokens":10}},"event_type":"interaction.completed"}"#,
		),
	];

	for (event_name, data) in cases {
		// -- Exec & Check
		assert_eq!(
			EnrichedEvent::from_frame(&frame(event_name, data)),
			None,
			"for {event_name}: {data}"
		);
	}

	Ok(())
}

#[test]
fn test_ix_enriched_unknown_server_tool_passes_through() -> Result<()> {
	// -- Setup & Fixtures
	// An unverified server-side tool must surface with its payload rather than vanish.
	let data = r#"{"index":2,"delta":{"type":"code_execution_call","arguments":{"code":"print(1)"}},"event_type":"step.delta"}"#;

	// -- Exec
	let event = EnrichedEvent::from_frame(&frame("step.delta", data));

	// -- Check
	let Some(EnrichedEvent::ServerTool { kind, payload }) = event else {
		return Err(format!("expected a ServerTool passthrough, got {event:?}").into());
	};
	assert_eq!(kind, "code_execution_call");
	assert_eq!(payload["arguments"]["code"], "print(1)");

	Ok(())
}

#[test]
fn test_ix_enriched_malformed_frames_are_not_fatal() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("step.delta", "not json at all"),
		("step.delta", r#"{"index":0}"#),
		("step.start", r#"{"index":0,"step":{}}"#),
		(
			"step.delta",
			r#"{"delta":{"type":"text_annotation_delta","annotations":[]}}"#,
		),
	];

	for (event_name, data) in cases {
		// -- Exec & Check
		assert_eq!(EnrichedEvent::from_frame(&frame(event_name, data)), None, "for {data}");
	}

	// A frame with no SSE event name (non-SSE transports) is simply not ours.
	let no_event = RawFrameRef {
		index: 0,
		event: None,
		data: r#"{"delta":{"type":"google_search_call"}}"#,
		elapsed_us: 0,
	};
	assert_eq!(EnrichedEvent::from_frame(&no_event), None);

	Ok(())
}
