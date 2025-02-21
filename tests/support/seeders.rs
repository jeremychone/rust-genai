use genai::chat::{ChatMessage, ChatRequest, ContentPart, ImageSource, Tool};
use genai::embed::{BatchEmbedRequest, SingleEmbedRequest};
use serde_json::json;

pub fn seed_chat_req_simple() -> ChatRequest {
	ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	])
}

pub fn seed_embed_req_single() -> SingleEmbedRequest {
	SingleEmbedRequest::new("This should be a single embed request")
}

pub fn seed_embed_req_batch() -> BatchEmbedRequest {
	BatchEmbedRequest::new(vec![
		"This should be a batch embed request",
		"There should be three documents in total",
		"This is the third one",
	])
}

pub fn seed_chat_req_tool_simple() -> ChatRequest {
	ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::user("What is the temperature in C, in Paris, France"),
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
