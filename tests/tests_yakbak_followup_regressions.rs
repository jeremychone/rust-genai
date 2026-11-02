//! HTTP replay assertions for provider regressions merged after #348.

mod support;

use genai::chat::{
	ChatMessage, ChatOptions, ChatRequest, ChatResponseFormat, ContentPart, JsonSpec, MessageContent, ReasoningEffort,
	StopReason, ToolCall, ToolChoice, ToolResponse,
};
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

// #364: all thought parts in a frame must be retained when only reasoning capture is requested.
#[tokio::test]
async fn gemini_stream_captures_every_thought_part() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "multi_thought_capture_regression").await?;
	let options = ChatOptions::default().with_capture_reasoning_content(true);
	let result = client
		.exec_chat_stream("gemini-2.5-flash", ChatRequest::from_user("hi"), Some(&options))
		.await?;
	let end = extract_stream_end(result.stream).await?.stream_end;
	assert_eq!(
		end.captured_reasoning_content.as_deref(),
		Some("First thought. Second thought.")
	);
	assert!(end.captured_content.is_none());
	Ok(())
}

// #363: a server tool block and a citation delta must not interrupt streamed text.
#[tokio::test]
async fn anthropic_stream_skips_web_search_metadata() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "web_search_delta_regression").await?;
	let result = client
		.exec_chat_stream(
			"anthropic::claude-opus-5-5",
			ChatRequest::from_user("hi"),
			Some(&ChatOptions::default().with_capture_content(true)),
		)
		.await?;
	let end = extract_stream_end(result.stream).await?;
	assert_eq!(end.content.as_deref(), Some("Let me search. Rust is a language."));
	Ok(())
}

// #358: incomplete_details is the stop reason, in both response modes.
#[tokio::test]
async fn responses_incomplete_reason_nonstream() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "incomplete_reason_nonstream_regression").await?;
	let result = client
		.exec_chat("openai_resp::gpt-6-luna", ChatRequest::from_user("hi"), None)
		.await?;
	assert!(matches!(result.stop_reason, Some(StopReason::ContentFilter(_))));
	Ok(())
}

#[tokio::test]
async fn responses_incomplete_reason_stream() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "incomplete_reason_stream_regression").await?;
	let result = client
		.exec_chat_stream("openai_resp::gpt-6-luna", ChatRequest::from_user("hi"), None)
		.await?;
	let end = extract_stream_end(result.stream).await?.stream_end;
	assert!(matches!(end.captured_stop_reason, Some(StopReason::ContentFilter(_))));
	Ok(())
}

// #365: Anthropic's thinking-token count survives stream usage normalization.
#[tokio::test]
async fn anthropic_stream_captures_thinking_usage() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "thinking_usage_regression").await?;
	let result = client
		.exec_chat_stream(
			"anthropic::claude-opus-5-5",
			ChatRequest::from_user("hi"),
			Some(&ChatOptions::default().with_capture_usage(true)),
		)
		.await?;
	let end = extract_stream_end(result.stream).await?.stream_end;
	let usage = end.captured_usage.ok_or("usage")?;
	assert_eq!(
		usage.completion_tokens_details.and_then(|d| d.reasoning_tokens),
		Some(7)
	);
	Ok(())
}

// #357: cached Bedrock input tokens contribute to the prompt total.
#[tokio::test]
async fn bedrock_counts_cached_tokens() -> TestResult<()> {
	let (client, _server) = replay_client("bedrock", "cached_input_regression").await?;
	let result = client
		.exec_chat(
			"bedrock_api::global.openai.gpt-5.6-terra",
			ChatRequest::from_user("hi"),
			None,
		)
		.await?;
	assert_eq!(result.usage.prompt_tokens, Some(95));
	assert_eq!(
		result.usage.prompt_tokens_details.as_ref().and_then(|d| d.cached_tokens),
		Some(7)
	);
	Ok(())
}

// #370: Responses rejects stop and seed in its request body.
#[tokio::test]
async fn responses_omits_unsupported_options() -> TestResult<()> {
	let (client, server) = replay_client("openai_resp", "unsupported_options_regression").await?;
	let options = ChatOptions::default().with_stop_sequence("END").with_seed(42);
	client
		.exec_chat("openai_resp::gpt-6-luna", ChatRequest::from_user("hi"), Some(&options))
		.await?;
	let requests = server.replay_requests().await;
	assert!(requests[0].get("stop").is_none());
	assert!(requests[0].get("seed").is_none());
	Ok(())
}

// #371: newer Claude models reject temperature and top_p.
#[tokio::test]
async fn anthropic_omits_rejected_sampling_options() -> TestResult<()> {
	let (client, server) = replay_client("anthropic", "sampling_options_regression").await?;
	let options = ChatOptions::default().with_temperature(0.7).with_top_p(0.8);
	client
		.exec_chat(
			"anthropic::claude-opus-5-5",
			ChatRequest::from_user("hi"),
			Some(&options),
		)
		.await?;
	let requests = server.replay_requests().await;
	assert!(requests[0].get("temperature").is_none());
	assert!(requests[0].get("top_p").is_none());
	Ok(())
}

// #362: Ollama takes the schema object directly as format.
#[tokio::test]
async fn ollama_sends_json_schema_format() -> TestResult<()> {
	let (client, server) = replay_client("ollama", "schema_format_regression").await?;
	let schema = json!({"type":"object","properties":{"ok":{"type":"boolean"}}});
	let options = ChatOptions::default()
		.with_response_format(ChatResponseFormat::JsonSpec(JsonSpec::new("result", schema.clone())));
	client
		.exec_chat("ollama::llama3.2:3b", ChatRequest::from_user("hi"), Some(&options))
		.await?;
	let requests = server.replay_requests().await;
	assert_eq!(requests[0]["format"], schema);
	Ok(())
}

// #361: tool results precede free text in an Anthropic tool message.
#[tokio::test]
async fn anthropic_tool_result_precedes_text() -> TestResult<()> {
	let (client, server) = replay_client("anthropic", "tool_result_order_regression").await?;
	let content = MessageContent::from_parts(vec![
		ContentPart::Text("Additional context".into()),
		ContentPart::ToolResponse(ToolResponse::new("toolu_1", "Sunny")),
	]);
	let request = ChatRequest::new(vec![ChatMessage::user("Weather?"), ChatMessage::tool(content)]);
	client.exec_chat("anthropic::claude-haiku-4-5", request, None).await?;
	let requests = server.replay_requests().await;
	let blocks = requests[0]["messages"][1]["content"].as_array().ok_or("tool blocks")?;
	assert_eq!(blocks[0]["type"], "tool_result");
	assert_eq!(blocks[1]["type"], "text");
	Ok(())
}

// #351: Cohere v1 requires `message` in each chat_history entry.
#[tokio::test]
async fn cohere_history_uses_message_field() -> TestResult<()> {
	let (client, server) = replay_client("cohere", "history_message_regression").await?;
	let request = ChatRequest::new(vec![
		ChatMessage::user("What is Rust?"),
		ChatMessage::assistant("A systems language."),
		ChatMessage::user("Who made it?"),
	]);
	client.exec_chat("cohere::command-r-plus", request, None).await?;
	let requests = server.replay_requests().await;
	assert_eq!(requests[0]["chat_history"][0]["message"], "What is Rust?");
	assert_eq!(requests[0]["chat_history"][1]["message"], "A systems language.");
	Ok(())
}

// #373: Gemini 4 uses thinkingLevel and gives its first unsigned call a validator signature.
#[tokio::test]
async fn gemini_4_thinking_level_and_first_call_signature() -> TestResult<()> {
	let (client, server) = replay_client("gemini", "generation_4_request_regression").await?;
	let call = ToolCall {
		call_id: "call_1".into(),
		fn_name: "weather".into(),
		fn_arguments: json!({"city":"Paris"}),
		thought_signatures: None,
	};
	let request = ChatRequest::from_user("Weather?")
		.append_message(ChatMessage::assistant(MessageContent::from_tool_calls(vec![call])));
	let options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::High);
	client.exec_chat("gemini::gemini-4-argon", request, Some(&options)).await?;
	let requests = server.replay_requests().await;
	assert_eq!(
		requests[0]["generationConfig"]["thinkingConfig"]["thinkingLevel"],
		"HIGH"
	);
	assert_eq!(
		requests[0]["contents"][1]["parts"][0]["thoughtSignature"],
		"skip_thought_signature_validator"
	);
	Ok(())
}

// #366: Vertex Claude forwards its tool, reasoning, format, and extra-body options.
#[tokio::test]
async fn vertex_claude_forwards_request_options() -> TestResult<()> {
	let (client, server) = replay_client("vertex", "claude_options_regression").await?;
	let schema = json!({"type":"object","properties":{"answer":{"type":"string"}}});
	let options = ChatOptions::default()
		.with_tool_choice(ToolChoice::Required)
		.with_reasoning_effort(ReasoningEffort::High)
		.with_response_format(JsonSpec::new("answer", schema))
		.with_extra_body(json!({"custom_marker":"forwarded"}));
	client
		.exec_chat(
			"vertex::claude-sonnet-4-6",
			ChatRequest::from_user("hi"),
			Some(&options),
		)
		.await?;
	let requests = server.replay_requests().await;
	let body = &requests[0];
	assert_eq!(body["tool_choice"], json!({"type":"any"}));
	assert_eq!(body["output_config"]["format"]["type"], "json_schema");
	assert!(body["output_config"].get("effort").is_some());
	assert_eq!(body["custom_marker"], "forwarded");
	assert!(body.get("model").is_none());
	Ok(())
}
