//! Recording scripts for yakbak cassettes.
//!
//! These are `#[ignore]` tests — run manually with real API keys.
//! Each provider's keys and base URLs are independent; you only need
//! the credentials for the provider(s) you want to record.
//!
//! ```sh
//! # Record all providers (need all keys):
//! OPENAI_API_KEY=... GEMINI_API_KEY=... GITHUB_TOKEN=... OLLAMA_API_KEY=... cargo test --test tests_yakbak_record -- --ignored
//!
//! # Record only Gemini scenarios:
//! GEMINI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_gemini
//!
//! # Record only Gemini Interactions scenarios:
//! GEMINI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_gemini_interactions
//!
//! # Record only OpenAI scenarios:
//! OPENAI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_openai
//!
//! # Record only GitHub Copilot scenarios:
//! GITHUB_TOKEN=... cargo test --test tests_yakbak_record -- --ignored record_github_copilot
//!
//! # Record only Ollama Cloud scenarios:
//! OLLAMA_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_ollama_cloud
//!
//! # Record an Anthropic thinking/tool scenario (endpoint and model may be overridden):
//! ANTHROPIC_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_anthropic_thinking_tool_stream
//!
//! # Record two turns of the public FFmpeg H.264 adjudication reproduction. Gateways that
//! # require their billing/identity system blocks and opaque metadata receive only those fields:
//! ANTHROPIC_SMOKE_EXTRA_BODY="$(jq -c '{system: .system[0:2], metadata, context_management}' /path/to/request.json)" \
//! ANTHROPIC_BASE_URL=... \
//! ANTHROPIC_THINKING_MODEL=anthropic::claude-opus-5 \
//! ANTHROPIC_API_KEY=... \
//! EXPERIMENTAL_BEARER_TOKEN=... \
//! cargo test --test tests_yakbak_record -- --ignored record_anthropic_adjudication_tool_stream
//!
//! # Record a single scenario by name:
//! GEMINI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored record_gemini_thinking_stream
//! ```
//!
//! Optional env vars for custom endpoints: `OPENAI_BASE_URL`, `GEMINI_BASE_URL`, `GITHUB_COPILOT_BASE_URL`, `OLLAMA_CLOUD_BASE_URL`.
//!
//! Each test records a response cassette to `tests/data/yakbak/{provider}/{scenario}/`.

mod support;

use genai::chat::*;
use serde_json::{Value, json};
use support::yakbak::record_client;
use support::{TestResult, extract_stream_end};

fn openai_backend() -> String {
	std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1/".to_string())
}

const OPENAI_MODEL: &str = "openai_resp::gpt-5.4-mini";

fn anthropic_backend() -> String {
	std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com/v1/".to_string())
}

fn anthropic_thinking_model() -> String {
	std::env::var("ANTHROPIC_THINKING_MODEL").unwrap_or_else(|_| "anthropic::claude-sonnet-4-5".to_string())
}

fn anthropic_thinking_tool_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("Use the provided tool when it is needed. Keep any visible answer concise."),
		ChatMessage::user(
			"Determine which of Berlin, Cairo, and Paris is in Africa, then call get_weather for that city in Celsius.",
		),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": { "type": "string" },
			"country": { "type": "string" },
			"unit": { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"]
	})))
}

fn anthropic_adjudication_seed() -> TestResult<Value> {
	serde_json::from_str(include_str!(
		"data/recording_inputs/anthropic_adjudication_two_turn_seed.json"
	))
	.map_err(|err| format!("parse adjudication recording seed: {err}").into())
}

fn anthropic_adjudication_tool_request(seed: &Value) -> TestResult<ChatRequest> {
	let prompt = seed
		.pointer("/messages/0/content")
		.and_then(Value::as_str)
		.ok_or("adjudication seed is missing messages[0].content")?;
	let tool_values = seed
		.get("tools")
		.and_then(Value::as_array)
		.ok_or("adjudication seed is missing tools")?;
	let tools = tool_values
		.iter()
		.map(|tool| {
			let name = tool.get("name").and_then(Value::as_str).ok_or("seed tool is missing name")?;
			let description = tool
				.get("description")
				.and_then(Value::as_str)
				.ok_or("seed tool is missing description")?;
			let schema = tool.get("schema").cloned().ok_or("seed tool is missing schema")?;
			Ok(Tool::new(name).with_description(description).with_schema(schema))
		})
		.collect::<TestResult<Vec<_>>>()?;

	Ok(ChatRequest::new(vec![ChatMessage::user(prompt)]).with_tools(tools))
}

fn anthropic_adjudication_tool_result(seed: &Value, tool_call: &ToolCall) -> TestResult<String> {
	let results = seed
		.get("tool_results")
		.and_then(Value::as_array)
		.ok_or("adjudication seed is missing tool_results")?;

	if let Some(result) = results.iter().find(|result| {
		result.get("name").and_then(Value::as_str) == Some(tool_call.fn_name.as_str())
			&& result.get("arguments") == Some(&tool_call.fn_arguments)
	}) {
		return result
			.get("result")
			.and_then(Value::as_str)
			.map(str::to_string)
			.ok_or_else(|| "seed tool result is missing result text".into());
	}

	match tool_call.fn_name.as_str() {
		"Read" => {
			let requested_path = tool_call
				.fn_arguments
				.get("path")
				.and_then(Value::as_str)
				.ok_or("Read call is missing path")?;
			let requested_offset = tool_call
				.fn_arguments
				.get("offset")
				.and_then(Value::as_u64)
				.ok_or("Read call is missing offset")?;
			let requested_limit = tool_call
				.fn_arguments
				.get("limit")
				.and_then(Value::as_u64)
				.ok_or("Read call is missing limit")?;
			let requested_end = requested_offset.checked_add(requested_limit).ok_or("Read range overflows")?;

			let mut source_lines = std::collections::BTreeMap::new();
			for result in results.iter().filter(|result| {
				result.get("name").and_then(Value::as_str) == Some("Read")
					&& result.pointer("/arguments/path").and_then(Value::as_str) == Some(requested_path)
			}) {
				if let Some(text) = result.get("result").and_then(Value::as_str) {
					for line in text.lines() {
						if let Some(line_number) =
							line.split_whitespace().next().and_then(|value| value.parse::<u64>().ok())
						{
							source_lines.insert(line_number, line);
						}
					}
				}
			}

			let selected = (requested_offset..requested_end)
				.map(|line_number| source_lines.get(&line_number).copied())
				.collect::<Option<Vec<_>>>()
				.ok_or_else(|| {
					format!(
						"no complete public-source Read result for {requested_path}:{requested_offset}+{requested_limit}"
					)
				})?;
			Ok(selected.join("\n"))
		}
		"Grep" => Ok("libavcodec/h264_picture.c:231:    h->current_slice = 0;\n\
libavcodec/h264dec.c:472:    h->current_slice = 0;\n\
libavcodec/h264dec.c:595:        h->current_slice = 0;\n\
libavcodec/h264_slice.c:1982:    sl->slice_num = ++h->current_slice;"
			.to_string()),
		"Submit" => Ok("Submission recorded.".to_string()),
		other => Err(format!("unexpected tool call in two-turn reproduction: {other}").into()),
	}
}

fn anthropic_adjudication_options() -> TestResult<ChatOptions> {
	let mut options = ChatOptions::default()
		.with_max_tokens(4096)
		.with_reasoning_effort(ReasoningEffort::High)
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);
	if let Ok(raw) = std::env::var("ANTHROPIC_SMOKE_EXTRA_BODY") {
		let extra_body: Value = serde_json::from_str(&raw)
			.map_err(|err| format!("ANTHROPIC_SMOKE_EXTRA_BODY must be valid JSON: {err}"))?;
		options = options.with_extra_body(extra_body);
	}
	if let Ok(token) = std::env::var("EXPERIMENTAL_BEARER_TOKEN") {
		options = options.with_extra_headers(("authorization", format!("Bearer {token}")));
	}
	Ok(options)
}

#[tokio::test]
#[ignore]
async fn record_anthropic_thinking_tool_stream() -> TestResult<()> {
	let (client, mut server) = record_client("anthropic", "thinking_tool_stream", &anthropic_backend()).await?;

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);

	let model = anthropic_thinking_model();
	let initial_request = anthropic_thinking_tool_request();
	let continuation_request = initial_request.clone();
	let stream_res = client.exec_chat_stream(&model, initial_request, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Reasoning chunks combined: {} bytes",
		extract.reasoning_content.as_deref().map(str::len).unwrap_or(0)
	);
	eprintln!(
		"[record] Signature deltas: count={}, lengths={:?}",
		extract.thought_signature_chunks.len(),
		extract.thought_signature_chunks.iter().map(String::len).collect::<Vec<_>>()
	);
	eprintln!(
		"[record] Tool calls: {:?}",
		extract.stream_end.captured_tool_calls().as_ref().map(|calls| calls.len())
	);

	let tool_call = extract
		.stream_end
		.captured_tool_calls()
		.and_then(|calls| calls.first().cloned().cloned())
		.ok_or("recorded response should contain a tool call")?;
	let tool_response = ToolResponse::from_tool_call(&tool_call, "25 C and clear");
	let continuation_request = continuation_request.append_tool_use_from_stream_end(&extract.stream_end, tool_response);
	let continuation_res = client.exec_chat_stream(&model, continuation_request, Some(&options)).await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	eprintln!(
		"[record] Continuation text: {} bytes, reasoning: {} bytes, signature deltas: {}",
		continuation.content.as_deref().map(str::len).unwrap_or(0),
		continuation.reasoning_content.as_deref().map(str::len).unwrap_or(0),
		continuation.thought_signature_chunks.len(),
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_anthropic_adjudication_tool_stream() -> TestResult<()> {
	let (client, mut server) =
		record_client("anthropic", "thinking_adjudication_tool_stream", &anthropic_backend()).await?;
	let seed = anthropic_adjudication_seed()?;
	let options = anthropic_adjudication_options()?;
	let model = anthropic_thinking_model();
	let initial_request = anthropic_adjudication_tool_request(&seed)?;
	let continuation_base = initial_request.clone();

	// Turn 1: ask the public FFmpeg adjudication question and capture the
	// provider's canonical assistant content, including signed thinking.
	let stream_res = client.exec_chat_stream(&model, initial_request, Some(&options)).await?;
	let first = extract_stream_end(stream_res.stream).await?;
	let tool_calls: Vec<ToolCall> = first
		.stream_end
		.captured_tool_calls()
		.ok_or("the first adjudication turn did not call an evidence tool")?
		.into_iter()
		.cloned()
		.collect();
	if tool_calls.is_empty() {
		return Err("the first adjudication turn did not call an evidence tool".into());
	}
	eprintln!(
		"[record] Adjudication turn 1: text={} bytes, reasoning={} bytes, signatures={}, tool_calls={}",
		first.content.as_deref().map(str::len).unwrap_or(0),
		first.reasoning_content.as_deref().map(str::len).unwrap_or(0),
		first.thought_signature_chunks.len(),
		tool_calls.len(),
	);

	// Return one public-source result for every tool call. The first helper
	// appends the captured assistant turn exactly once; the rest are sibling
	// tool-result messages for the same assistant turn.
	let first_call = &tool_calls[0];
	let first_result = anthropic_adjudication_tool_result(&seed, first_call)?;
	let mut continuation_request = continuation_base.append_tool_use_from_stream_end(
		&first.stream_end,
		ToolResponse::from_tool_call(first_call, first_result),
	);
	for tool_call in &tool_calls[1..] {
		let result = anthropic_adjudication_tool_result(&seed, tool_call)?;
		continuation_request = continuation_request.append_message(ToolResponse::from_tool_call(tool_call, result));
	}

	// Turn 2: record exactly one response to the evidence, then stop.
	let continuation_res = client.exec_chat_stream(&model, continuation_request, Some(&options)).await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	eprintln!(
		"[record] Adjudication turn 2: text={} bytes, reasoning={} bytes, signatures={}, tool_calls={}",
		continuation.content.as_deref().map(str::len).unwrap_or(0),
		continuation.reasoning_content.as_deref().map(str::len).unwrap_or(0),
		continuation.thought_signature_chunks.len(),
		continuation
			.stream_end
			.captured_tool_calls()
			.as_ref()
			.map(|calls| calls.len())
			.unwrap_or(0),
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_anthropic_thinking_tool_non_stream() -> TestResult<()> {
	let (client, mut server) = record_client("anthropic", "thinking_tool_non_stream", &anthropic_backend()).await?;

	let model = anthropic_thinking_model();
	let response = client.exec_chat(&model, anthropic_thinking_tool_request(), None).await?;
	eprintln!(
		"[record] Non-stream reasoning: {} bytes, tool calls: {}",
		response.reasoning_content.as_deref().map(str::len).unwrap_or(0),
		response.tool_calls().len()
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_stream() -> TestResult<()> {
	let (client, mut server) = record_client("openai_resp", "reasoning_stream", &openai_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);
	eprintln!(
		"[record] Stream reasoning: {:?}",
		extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_summary_capture() -> TestResult<()> {
	// Regression for the two-part fix. Pairs effort=Low with
	// capture_reasoning_content=true so the API actually emits
	// summary deltas (effort alone with no capture gets no summary;
	// capture alone gets effort="none" server-default → no reasoning
	// at all; both required IN PRACTICE on current models). Once
	// emitted, the `response.reasoning_summary_text.delta` events
	// must land in `captured_reasoning_content` — previously the
	// streamer only parsed the `response.reasoning_text.delta`
	// family and silently dropped summaries.
	let (client, mut server) = record_client("openai_resp", "reasoning_summary_capture", &openai_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer concisely."),
		ChatMessage::user("Why is 47 * 23 = 1081? Reason step by step."),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_aihubmix_chat_stream() -> TestResult<()> {
	let (client, mut server) = record_client("aihubmix", "chat_stream", &aihubmix_backend()).await?;

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'hello' and nothing else.")]);
	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client.exec_chat_stream(AIHUBMIX_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_stream_tools() -> TestResult<()> {
	let (client, mut server) = record_client("openai_resp", "reasoning_stream_tools", &openai_backend()).await?;

	let chat_req = seed_tool_request();
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!("[record] Stream reasoning: {:?}", extract.reasoning_content.is_some());
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_custom_grammar_tool() -> TestResult<()> {
	// Records the freeform custom-tool (lark grammar) flow:
	// request 1 — model answers with a `custom_tool_call` item whose `input`
	// is a raw (non-JSON) patch constrained by the grammar; request 2 — the
	// tool result goes back as `custom_tool_call_output` and the model
	// produces the final text.
	let (client, mut server) = record_client("openai_resp", "custom_grammar_tool", &openai_backend()).await?;

	let chat_req = seed_apply_patch_request();
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	// -- Turn 1: expect a custom tool call with the raw patch as input
	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req.clone(), Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	let tool_calls = extract
		.stream_end
		.captured_into_tool_calls()
		.ok_or("Should have captured a custom tool call")?;
	for tc in &tool_calls {
		eprintln!("[record] Tool call: {} ({})", tc.fn_name, tc.call_id);
		eprintln!(
			"[record] Input:\n{}",
			tc.fn_arguments.as_str().unwrap_or("<not a string>")
		);
	}

	// -- Turn 2: send the tool output back, get the final text
	let call_id = tool_calls[0].call_id.clone();
	let chat_req = chat_req
		.append_message(ChatMessage::from(tool_calls))
		.append_message(ToolResponse::new(call_id, "Patch applied successfully."));

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Final content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(120)])
	);

	server.shutdown().await;
	Ok(())
}

/// The OpenAI `apply_patch` freeform custom tool, grammar-constrained (lark).
fn apply_patch_tool() -> Tool {
	Tool::new("apply_patch")
		.with_description(
			"Use the `apply_patch` tool to edit files. This is a FREEFORM tool, so do not wrap the patch in JSON.",
		)
		.with_custom_format(json!({
			"type": "grammar",
			"syntax": "lark",
			"definition": APPLY_PATCH_GRAMMAR,
		}))
}

const APPLY_PATCH_GRAMMAR: &str = r#"start: begin_patch hunk+ end_patch
begin_patch: "*** Begin Patch" LF
end_patch: "*** End Patch" LF?

hunk: add_hunk | delete_hunk | update_hunk
add_hunk: "*** Add File: " filename LF add_line+
delete_hunk: "*** Delete File: " filename LF
update_hunk: "*** Update File: " filename LF change_move? change?

filename: /(.+)/
add_line: "+" /(.*)/ LF -> line

change_move: "*** Move to: " filename LF
change: (change_context | change_line)+ eof_line?
change_context: ("@@" | "@@ " /(.+)/) LF
change_line: ("+" | "-" | " ") /(.*)/ LF
eof_line: "*** End of File" LF

%import common.LF
"#;

fn seed_apply_patch_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("You are a coding assistant. Edit files with the apply_patch tool."),
		ChatMessage::user(
			"Rename the function `greet` to `welcome` in the file `hello.py` (update the call site too). \
			 Current content of hello.py:\n\n\
			 def greet(name):\n    print(f\"Hello, {name}!\")\n\ngreet(\"World\")\n",
		),
	])
	.append_tool(apply_patch_tool())
}

fn gemini_backend() -> String {
	std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/".to_string())
}

const GEMINI_MODEL: &str = "gemini-2.5-flash";
const GEMINI_TOOL_MODEL: &str = "gemini-3.1-pro-preview";
const GEMINI_URL_CONTEXT_MODEL: &str = "gemini-3.7-flash";

#[tokio::test]
#[ignore]
async fn record_gemini_thinking_stream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "thinking_stream", &gemini_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(GEMINI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);
	eprintln!(
		"[record] Stream reasoning: {:?}",
		extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_gemini_tool_stream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "tool_stream", &gemini_backend()).await?;

	// A reasoning-heavy prompt so the model emits `thought:true` summary parts
	// alongside the `functionCall` — exercises text + reasoning + tool-call
	// paths of the SSE streamer in a single cassette.
	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a thoughtful assistant. Always reason carefully before invoking tools."),
		ChatMessage::user(
			"Of these three cities — Berlin, Cairo, Paris — exactly one is in Africa. \
			 Reason carefully about which one, then call get_weather for that city in Celsius. \
			 Walk through your reasoning explicitly.",
		),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city":    { "type": "string", "description": "The city name" },
			"country": { "type": "string", "description": "The country" },
			"unit":    { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"],
	})));

	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::High)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true);

	let stream_res = client.exec_chat_stream(GEMINI_TOOL_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));
	eprintln!(
		"[record] Reasoning len: {:?}",
		extract.reasoning_content.as_deref().map(|s| s.len())
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_gemini_url_context_stream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "url_context_stream", &gemini_backend()).await?;

	let chat_req = ChatRequest::from_user(
		"Read https://blog.rust-lang.org/ and tell me the title of the most recent post. One sentence.",
	)
	.append_tool(Tool::new("urlContext").with_config(json!({})));

	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream(GEMINI_URL_CONTEXT_MODEL, chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	eprintln!("[record] usage: {:?}", extract.stream_end.captured_usage);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_gemini_builtin_with_functions() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "builtin_with_functions", &gemini_backend()).await?;

	let chat_req = ChatRequest::from_user("Use the get_weather tool to get the current weather in Cairo, in Celsius.")
		.append_tool(Tool::new_web_search().with_config(WebSearchConfig::default()))
		.append_tool(Tool::new("urlContext").with_config(json!({})))
		.append_tool(
			Tool::new("get_weather")
				.with_description("Get the current weather for a city")
				.with_schema(json!({
					"type": "object",
					"properties": {
						"city": { "type": "string" },
						"unit": { "type": "string", "enum": ["C", "F"] }
					},
					"required": ["city", "unit"],
				})),
		);

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream(GEMINI_URL_CONTEXT_MODEL, chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	eprintln!(
		"[record] tool calls: {:?}",
		extract.stream_end.captured_tool_calls().map(|tc| tc.len())
	);
	eprintln!("[record] usage: {:?}", extract.stream_end.captured_usage);

	server.shutdown().await;
	Ok(())
}

fn github_copilot_backend() -> String {
	std::env::var("GITHUB_COPILOT_BASE_URL").unwrap_or_else(|_| "https://models.github.ai/inference/".to_string())
}

const GITHUB_COPILOT_MODEL: &str = "github_copilot::openai/gpt-4.1-mini";

#[tokio::test]
#[ignore]
async fn record_github_copilot_simple_stream() -> TestResult<()> {
	let (client, mut server) = record_client("github_copilot", "simple_stream", &github_copilot_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_capture_content(true);

	let stream_res = client.exec_chat_stream(GITHUB_COPILOT_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_github_copilot_tool_stream() -> TestResult<()> {
	let (client, mut server) = record_client("github_copilot", "tool_stream", &github_copilot_backend()).await?;

	let chat_req = seed_tool_request();
	let options = ChatOptions::default().with_capture_content(true).with_capture_tool_calls(true);

	let stream_res = client.exec_chat_stream(GITHUB_COPILOT_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));

	server.shutdown().await;
	Ok(())
}

fn seed_tool_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What is the temperature in C and weather, in Paris, France"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": {
				"type": "string",
				"description": "The city name"
			},
			"country": {
				"type": "string",
				"description": "The most likely country of this city name"
			},
			"unit": {
				"type": "string",
				"enum": ["C", "F"],
				"description": "The temperature unit of the country. C for Celsius, and F for Fahrenheit"
			}
		},
		"required": ["city", "country", "unit"],
	})))
}

fn ollama_cloud_backend() -> String {
	std::env::var("OLLAMA_CLOUD_BASE_URL").unwrap_or_else(|_| "https://ollama.com/".to_string())
}

fn aihubmix_backend() -> String {
	std::env::var("AIHUBMIX_BASE_URL").unwrap_or_else(|_| "https://aihubmix.com/v1/".to_string())
}

const OLLAMA_CLOUD_MODEL: &str = "ollama_cloud::gemma3:4b";

const AIHUBMIX_MODEL: &str = "aihubmix::gpt-4o-mini";

#[tokio::test]
#[ignore]
async fn record_ollama_cloud_simple_stream() -> TestResult<()> {
	let (client, mut server) = record_client("ollama_cloud", "simple_stream", &ollama_cloud_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_capture_content(true);

	let stream_res = client.exec_chat_stream(OLLAMA_CLOUD_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!(
		"[record] Stream content: {:?}",
		extract.content.as_deref().map(|s| &s[..s.len().min(80)])
	);

	server.shutdown().await;
	Ok(())
}

// region:    --- Gemini Interactions

const GEMINI_IX_MODEL: &str = "gemini-3.5-flash";

/// Shared seed for the tool scenarios: a reasoning-heavy prompt so the model emits a `thought`
/// step (and its signature) alongside the `function_call`.
fn gemini_ix_tool_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("You are a thoughtful assistant. Always reason carefully before invoking tools."),
		ChatMessage::user(
			"Of these three cities — Berlin, Cairo, Paris — exactly one is in Africa. \
			 Reason carefully about which one, then call get_weather for that city in Celsius.",
		),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city":    { "type": "string", "description": "The city name" },
			"country": { "type": "string", "description": "The country" },
			"unit":    { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"],
	})))
}

/// Streaming tool call. This is the cassette with the most bespoke logic behind it: the
/// Interactions stream delivers a function call across three event types — `step.start` carries
/// the name and id, `step.delta`/`arguments_delta` streams the arguments as a partial JSON
/// *string* that must be accumulated, and `step.stop` is where the call can finally be assembled.
/// `interaction.completed` carries no steps at all, so nothing can be recovered at the end.
#[tokio::test]
#[ignore]
async fn record_gemini_interactions_tool_stream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini_interactions", "tool_stream", &gemini_backend()).await?;

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream(GEMINI_IX_MODEL, gemini_ix_tool_request(), Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	eprintln!(
		"[record] Tool stream: reasoning={} bytes, signatures={}, tool_calls={}, response_id={:?}",
		extract.reasoning_content.as_deref().map(str::len).unwrap_or(0),
		extract.thought_signature_chunks.len(),
		extract.stream_end.captured_tool_calls().map(|c| c.len()).unwrap_or(0),
		extract.stream_end.captured_response_id,
	);

	server.shutdown().await;
	Ok(())
}

/// Two-turn tool round trip, non-streaming. Locks in the two request-shaping rules that the API
/// reference does not state and that both returned a bare 400 until they were found:
/// a replayed `function_call` must be preceded by its `thought` step, and `function_result.name`
/// is required even though the schema marks it optional.
#[tokio::test]
#[ignore]
async fn record_gemini_interactions_tool_full_flow() -> TestResult<()> {
	let (client, mut server) = record_client("gemini_interactions", "tool_full_flow", &gemini_backend()).await?;

	let chat_req = gemini_ix_tool_request();

	// -- Turn 1: get the tool call (and the thought signature riding on it).
	let chat_res = client.exec_chat(GEMINI_IX_MODEL, chat_req.clone(), None).await?;
	let tool_calls = chat_res.into_tool_calls();
	let first_tool_call = tool_calls.first().ok_or("turn 1 should have called get_weather")?;
	eprintln!(
		"[record] Turn 1: tool_calls={}, signatures_on_first_call={:?}",
		tool_calls.len(),
		first_tool_call.thought_signatures.as_ref().map(Vec::len),
	);

	// -- Turn 2: answer it. `ToolResponse::new` carries no fn_name on purpose — the adapter has
	//    to recover the name from the matching function_call.
	let tool_response = ToolResponse::new(
		&first_tool_call.call_id,
		r#"{"weather": "Sunny", "temperature": "32C"}"#,
	);
	let chat_req = chat_req.append_message(tool_calls.clone()).append_message(tool_response);

	let chat_res = client.exec_chat(GEMINI_IX_MODEL, chat_req, None).await?;
	eprintln!("[record] Turn 2: {:?}", chat_res.first_text());

	server.shutdown().await;
	Ok(())
}

/// Two turns of server-side history — the reason this adapter exists. Turn 2 sends only the new
/// message plus the previous interaction id; the transcript never leaves the server.
/// Also captures the `store: true` response shape, which is the only one carrying an `id`.
#[tokio::test]
#[ignore]
async fn record_gemini_interactions_stateful_session() -> TestResult<()> {
	let (client, mut server) = record_client("gemini_interactions", "stateful_session", &gemini_backend()).await?;

	// -- Turn 1: `store` must be explicit — the adapter never sets it implicitly.
	let chat_req = ChatRequest::from_user("My favorite language is Rust. Reply with just 'noted'.").with_store(true);
	let res_1 = client.exec_chat(GEMINI_IX_MODEL, chat_req, None).await?;
	let interaction_id = res_1.response_id.clone().ok_or("a stored interaction should have an id")?;
	eprintln!("[record] Turn 1: {:?} id={interaction_id}", res_1.first_text());

	// -- Turn 2: no history resent.
	let chat_req = ChatRequest::from_user("What is my favorite language?")
		.with_previous_response_id(&interaction_id)
		.with_store(true);
	let res_2 = client.exec_chat(GEMINI_IX_MODEL, chat_req, None).await?;
	eprintln!("[record] Turn 2: {:?}", res_2.first_text());

	server.shutdown().await;
	Ok(())
}

// endregion: --- Gemini Interactions
