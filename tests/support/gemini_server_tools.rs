//! Shared tool-combination round trip for the Gemini yakbak recorder and replay tests:
//! a built-in tool (`urlContext`) mixed with a function declaration, carried through a
//! full tool loop. Recorder and replay must send the same requests, so both use this.
//! See https://ai.google.dev/gemini-api/docs/generate-content/tool-combination

use super::{TestResult, extract_stream_end};
use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, MessageContent, Tool, ToolResponse};
use serde_json::json;

/// Max model turns before giving up on a final text answer.
const MAX_TURNS: usize = 5;

pub fn server_tools_request() -> ChatRequest {
	let url_context = Tool::new("urlContext").with_config(json!({}));
	let record_title = Tool::new("record_title")
		.with_description("Record the title of a web page that was read.")
		.with_schema(json!({
			"type": "object",
			"properties": {
				"url": { "type": "string" },
				"title": { "type": "string" }
			},
			"required": ["url", "title"]
		}));
	ChatRequest::from_user(
		"Read https://www.rust-lang.org with URL context, then call record_title with its URL and page title. \
		 After the tool result, reply with one short sentence.",
	)
	.with_tools(vec![url_context, record_title])
}

/// Run the tool loop until the model answers with text. Returns each turn's assistant
/// content, in order. Errors when the first turn has no function call (the round trip
/// would not be exercised) or the loop does not finish.
pub async fn run_server_tools_round_trip(
	client: &Client,
	model: &str,
	stream: bool,
) -> TestResult<Vec<MessageContent>> {
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true);
	let mut chat_req = server_tools_request();
	let mut turns = Vec::new();

	for turn in 0..MAX_TURNS {
		let content = if stream {
			let res = client.exec_chat_stream(model, chat_req.clone(), Some(&options)).await?;
			let extract = extract_stream_end(res.stream).await?;
			extract.stream_end.captured_content.ok_or("stream should capture content")?
		} else {
			client.exec_chat(model, chat_req.clone(), Some(&options)).await?.content
		};
		turns.push(content.clone());

		let calls: Vec<_> = content.tool_calls().into_iter().cloned().collect();
		if calls.is_empty() {
			if turn == 0 {
				return Err(format!("{model}: no function call on the first turn; round trip not exercised").into());
			}
			content.first_text().ok_or("final turn has no text")?;
			return Ok(turns);
		}
		chat_req = chat_req.append_message(ChatMessage::assistant(content));
		for call in calls {
			chat_req = chat_req.append_message(ToolResponse::new(call.call_id, json!({"recorded": true}).to_string()));
		}
	}
	Err(format!("{model}: no final text after {MAX_TURNS} turns").into())
}
