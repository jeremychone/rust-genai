use super::{IxDelta, IxStreamEvent};
use serde_json::json;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// The event fixtures below are copied from
// <https://ai.google.dev/gemini-api/docs/interactions/streaming>.

#[test]
fn test_gemini_ix_parses_interaction_created() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"interaction": {"id": "v1_abc", "status": "in_progress", "object": "interaction", "model": "gemini-3-flash-preview"},
		"event_type": "interaction.created"
	}))?;

	// -- Check
	assert!(
		matches!(event, IxStreamEvent::InteractionCreated { interaction } if interaction.id.as_deref() == Some("v1_abc")),
		"the id is captured here so an interrupted stream can still be continued"
	);

	Ok(())
}

#[test]
fn test_gemini_ix_parses_text_delta() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"index": 1,
		"delta": {"text": "1, 2, 3, 4, 5, 6, ", "type": "text"},
		"event_type": "step.delta"
	}))?;

	// -- Check
	assert!(matches!(
		event,
		IxStreamEvent::StepDelta { index: 1, delta: IxDelta::Text { text } } if text == "1, 2, 3, 4, 5, 6, "
	));

	Ok(())
}

#[test]
fn test_gemini_ix_parses_thought_signature_delta() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"index": 0,
		"delta": {"signature": "EpoGCpcGAXLI2nx", "type": "thought_signature"},
		"event_type": "step.delta"
	}))?;

	// -- Check
	assert!(matches!(
		event,
		IxStreamEvent::StepDelta { delta: IxDelta::ThoughtSignature { signature }, .. } if signature == "EpoGCpcGAXLI2nx"
	));

	Ok(())
}

#[test]
fn test_gemini_ix_parses_thought_summary_delta_with_nested_content() -> Result<()> {
	// -- Exec
	// NOTE: thought_summary nests its text one level deeper than every other delta.
	let event: IxStreamEvent = serde_json::from_value(json!({
		"index": 0,
		"delta": {"type": "thought_summary", "content": {"type": "text", "text": "I need to find the GCD..."}},
		"event_type": "step.delta"
	}))?;

	// -- Check
	let IxStreamEvent::StepDelta {
		delta: IxDelta::ThoughtSummary { content },
		..
	} = event
	else {
		return Err(format!("expected a thought_summary delta, got {event:?}").into());
	};
	assert_eq!(content["text"], "I need to find the GCD...");

	Ok(())
}

#[test]
fn test_gemini_ix_parses_arguments_delta() -> Result<()> {
	// -- Exec
	// Function-call arguments arrive as a partial JSON *string* that must be accumulated —
	// `step.start` only carries the name and id, and `interaction.completed` carries no steps.
	let event: IxStreamEvent = serde_json::from_value(json!({
		"index": 0,
		"delta": {"type": "arguments_delta", "arguments": "{\"location\": \"San Francisco, CA\"}"},
		"event_type": "step.delta"
	}))?;

	// -- Check
	assert!(matches!(
		event,
		IxStreamEvent::StepDelta { delta: IxDelta::ArgumentsDelta { arguments }, .. }
			if arguments == r#"{"location": "San Francisco, CA"}"#
	));

	Ok(())
}

#[test]
fn test_gemini_ix_parses_function_call_step_start() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"index": 0,
		"step": {"type": "function_call", "id": "un6k8t18", "name": "get_weather", "arguments": {}},
		"event_type": "step.start"
	}))?;

	// -- Check
	let IxStreamEvent::StepStart { index: 0, step } = event else {
		return Err(format!("expected a step.start, got {event:?}").into());
	};
	assert_eq!(step["type"], "function_call");
	assert_eq!(step["id"], "un6k8t18");
	assert_eq!(step["name"], "get_weather");

	Ok(())
}

#[test]
fn test_gemini_ix_parses_interaction_completed_with_usage() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"interaction": {
			"id": "v1_abc",
			"status": "completed",
			"usage": {
				"total_tokens": 346, "total_input_tokens": 11, "total_cached_tokens": 0,
				"total_output_tokens": 90, "total_tool_use_tokens": 0, "total_thought_tokens": 245
			},
			"object": "interaction",
			"model": "gemini-3-flash-preview"
		},
		"event_type": "interaction.completed"
	}))?;

	// -- Check
	let IxStreamEvent::InteractionCompleted { interaction } = event else {
		return Err(format!("expected interaction.completed, got {event:?}").into());
	};
	assert_eq!(interaction.id.as_deref(), Some("v1_abc"));
	assert_eq!(interaction.status.as_deref(), Some("completed"));
	// `interaction.completed` never carries steps — tool calls must come from step.stop.
	assert!(interaction.steps.is_empty());

	let usage = crate::chat::Usage::from(interaction.usage.ok_or("usage should be present")?);
	assert_eq!(usage.prompt_tokens, Some(11));
	assert_eq!(usage.completion_tokens, Some(90 + 245));
	assert_eq!(usage.total_tokens, Some(346));

	Ok(())
}

#[test]
fn test_gemini_ix_parses_error_event() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"error": {"message": "Deadline expired before operation could complete.", "code": "gateway_timeout"},
		"event_type": "error"
	}))?;

	// -- Check
	let IxStreamEvent::Error { error } = event else {
		return Err(format!("expected an error event, got {event:?}").into());
	};
	assert_eq!(error["code"], "gateway_timeout");

	Ok(())
}

#[test]
fn test_gemini_ix_unknown_events_are_not_fatal() -> Result<()> {
	// -- Setup & Fixtures
	// The Interactions API is in beta and adds event and delta types; neither may break a stream.
	let cases = [
		json!({"event_type": "interaction.some_event_invented_next_year", "whatever": true}),
		json!({"index": 0, "step": {"type": "google_search_call"}, "event_type": "step.start"}),
	];

	for case in cases {
		// -- Exec & Check
		let event: IxStreamEvent = serde_json::from_value(case.clone())?;
		if case["event_type"] == "interaction.some_event_invented_next_year" {
			assert!(matches!(event, IxStreamEvent::Unknown), "for {case}");
		}
	}

	// -- An unknown *delta* type is likewise absorbed
	let event: IxStreamEvent = serde_json::from_value(json!({
		"index": 0,
		"delta": {"type": "some_delta_invented_next_year", "data": "..."},
		"event_type": "step.delta"
	}))?;
	assert!(matches!(
		event,
		IxStreamEvent::StepDelta {
			delta: IxDelta::Other,
			..
		}
	));

	Ok(())
}

#[test]
fn test_gemini_ix_parses_status_update() -> Result<()> {
	// -- Exec
	let event: IxStreamEvent = serde_json::from_value(json!({
		"interaction_id": "v1_abc", "status": "in_progress", "event_type": "interaction.status_update"
	}))?;

	// -- Check
	assert!(matches!(
		event,
		IxStreamEvent::StatusUpdate { status } if status.as_deref() == Some("in_progress")
	));

	Ok(())
}
