//! Freeform custom tools (OpenAI Responses API) with a lark grammar constraint.
//!
//! Unlike JSON-schema function tools, a `custom` tool receives its input as a
//! single RAW string, optionally constrained by a grammar
//! (`{"type": "grammar", "syntax": "lark", "definition": "..."}`). This example
//! uses OpenAI's `apply_patch` tool: the model streams back a patch in the
//! constrained format, and the tool result goes back as a
//! `custom_tool_call_output` item.
//!
//! Run with (defaults to api.openai.com when `OPENAI_BASE_URL` is not set):
//! ```sh
//! OPENAI_API_KEY=... [OPENAI_BASE_URL=https://...] cargo run --example c24-tool-custom-grammar
//! ```

use futures::StreamExt;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, ReasoningEffort, Tool, ToolResponse};
use genai::resolver::{Endpoint, ServiceTargetResolver};
use genai::{Client, ServiceTarget};
use serde_json::json;

const MODEL: &str = "openai_resp::gpt-5.4-mini";

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// -- Build the client (honor OPENAI_BASE_URL for custom Responses-API backends)
	let mut client_builder = Client::builder();
	if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
		let base_url = if base_url.ends_with('/') {
			base_url
		} else {
			format!("{base_url}/")
		};
		client_builder = client_builder.with_service_target_resolver(ServiceTargetResolver::from_resolver_fn(
			move |st: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
				Ok(ServiceTarget {
					endpoint: Endpoint::from_owned(base_url.clone()),
					..st
				})
			},
		));
	}
	let client = client_builder.build();

	println!("--- Model: {MODEL}");

	// -- The freeform custom tool, grammar-constrained
	let apply_patch = Tool::new("apply_patch")
		.with_description(
			"Use the `apply_patch` tool to edit files. This is a FREEFORM tool, so do not wrap the patch in JSON.",
		)
		.with_custom_format(json!({
			"type": "grammar",
			"syntax": "lark",
			"definition": APPLY_PATCH_GRAMMAR,
		}));

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a coding assistant. Edit files with the apply_patch tool."),
		ChatMessage::user(
			"Rename the function `greet` to `welcome` in the file `hello.py` (update the call site too). \
			 Current content of hello.py:\n\n\
			 def greet(name):\n    print(f\"Hello, {name}!\")\n\ngreet(\"World\")\n",
		),
	])
	.append_tool(apply_patch);

	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_tool_calls(true);

	// -- Turn 1: stream the custom tool call (the raw patch)
	println!("--- Streaming the custom tool call");
	let mut chat_stream = client.exec_chat_stream(MODEL, chat_req.clone(), Some(&options)).await?;

	let mut tool_calls = Vec::new();
	while let Some(event) = chat_stream.stream.next().await {
		match event? {
			ChatStreamEvent::ToolCallChunk(chunk) => {
				// fn_arguments carries the raw input accumulated so far
				print!(
					"\r[input len: {}]",
					chunk.tool_call.fn_arguments.as_str().map_or(0, str::len)
				);
			}
			ChatStreamEvent::End(end) => {
				if let Some(captured) = end.captured_into_tool_calls() {
					tool_calls = captured;
				}
			}
			_ => (),
		}
	}
	println!();

	let Some(tool_call) = tool_calls.first() else {
		return Err("The model did not call apply_patch".into());
	};
	let patch = tool_call
		.fn_arguments
		.as_str()
		.ok_or("custom tool input should be a raw string")?;
	println!("--- apply_patch input (raw, grammar-constrained):\n{patch}");

	// -- Turn 2: send the tool output back, stream the final answer
	let call_id = tool_call.call_id.clone();
	let chat_req = chat_req
		.append_message(ChatMessage::from(tool_calls.clone()))
		.append_message(ToolResponse::new(call_id, "Patch applied successfully."));

	println!("--- Final response:");
	let mut chat_stream = client.exec_chat_stream(MODEL, chat_req, Some(&options)).await?;
	while let Some(event) = chat_stream.stream.next().await {
		if let ChatStreamEvent::Chunk(chunk) = event? {
			print!("{}", chunk.content);
		}
	}
	println!();

	Ok(())
}
