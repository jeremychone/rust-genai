use futures::StreamExt;
use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Tool, ToolResponse};
use genai::chat::{ChatStreamEvent, ToolCall};
use serde_json::json;

// const MODEL: &str = "gemini-2.0-flash";
const MODEL: &str = "deepseek-chat";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// 1. Define a tool for getting weather information
	let weather_tool = Tool::new("get_weather")
		.with_description("Get the current weather for a location")
		.with_schema(json!({
			"type": "object",
			"properties": {
				"city": {
					"type": "string",
					"description": "The city name"
				},
				"country": {
					"type": "string",
					"description": "The country of the city"
				},
				"unit": {
					"type": "string",
					"enum": ["C", "F"],
					"description": "Temperature unit (C for Celsius, F for Fahrenheit)"
				}
			},
			"required": ["city", "country", "unit"]
		}));

	// 2. Create initial chat request with the user query and the tool
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather like in Shenzhen, China?")])
		.with_tools(vec![weather_tool]);

	// 3. Set options to capture tool calls in the streaming response
	let chat_options = ChatOptions::default().with_capture_tool_calls(true);
	let print_options = PrintChatStreamOptions::from_print_events(false);

	// 4. Make the streaming call and handle the events
	let mut chat_stream = client.exec_chat_stream(MODEL, chat_req.clone(), Some(&chat_options)).await?;

	let mut tool_calls: Vec<ToolCall> = [].to_vec();
	// print_chat_stream(chat_res, Some(&print_options)).await?;
	println!("--- Streaming response with tool calls");
	while let Some(result) = chat_stream.stream.next().await {
		match result? {
			ChatStreamEvent::Start => {
				println!("Stream started");
			}
			ChatStreamEvent::Chunk(chunk) => {
				print!("{}", chunk.content);
			}
			ChatStreamEvent::ToolCallChunk(tool_chunk) => {
				println!(
					"\nTool Call: {} with args: {}",
					tool_chunk.tool_call.fn_name, tool_chunk.tool_call.fn_arguments
				);
			}
			ChatStreamEvent::ReasoningChunk(chunk) => {
				println!("\nReasoning: {}", chunk.content);
			}
			ChatStreamEvent::End(end) => {
				println!("\nStream ended");

				// Check if we captured any tool calls
				if let Some(captured_tool_calls) = end.captured_into_tool_calls() {
					println!("\nCaptured Tool Calls:");
					tool_calls = captured_tool_calls.clone();
					for tool_call in captured_tool_calls {
						println!("- Function: {}", tool_call.fn_name);
						println!("  Arguments: {}", tool_call.fn_arguments);
					}
				}
			}
		}
	}

	// 5. Now demonstrate how to handle the tool call and continue the conversation
	println!("\n--- Demonstrating full tool call workflow");
	if tool_calls.is_empty() {
		println!("No tool calls captured, cannot continue.");
		return Ok(());
	}
	// Simulate executing the function
	let first_tool_call = &tool_calls[0];
	let tool_response = ToolResponse::new(
		first_tool_call.call_id.clone(),
		json!({
			"temperature": 22.5,
			"condition": "Sunny",
			"humidity": 65
		})
		.to_string(),
	);

	// Add both the tool calls and response to chat history
	let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

	// Get final streaming response
	let chat_options = ChatOptions::default();
	let chat_res = client.exec_chat_stream(MODEL, chat_req, Some(&chat_options)).await?;

	print_chat_stream(chat_res, Some(&print_options)).await?;

	Ok(())
}
