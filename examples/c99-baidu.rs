//! Example: Baidu Qianfan (Wenxin Workshop) adapter with coding plan support
//!
//! This example demonstrates how to use the Baidu Qianfan adapter with various models
//! including coding plan with both OpenAI and Anthropic protocols.
//!
//! ## Setup
//!
//! 1. Get your Baidu Qianfan API key from https://console.bce.baidu.com/qianfan/
//! 2. Set environment variable: `export BAIDU_API_KEY=your_api_key`
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example c99-baidu
//! ```
//!
//! ## Coding Plan Namespaces
//!
//! - `baidu-coding-openai::` - Coding plan with OpenAI protocol
//! - `baidu-coding-anthropic::` - Coding plan with Anthropic protocol
//! - Regular models can be used with `baidu::` namespace or auto-detected

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use std::result::Result;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize tracing for logging
	tracing_subscriber::fmt::init();

	// Create a client with default configuration
	let client = Client::default();

	// Example 1: Using glm models (chat)
	println!("=== Example 1: Using glm-5 ===");
	let chat_request = ChatRequest::default().append_message(ChatMessage::user("Hello, how are you?"));

	match client.exec_chat("baidu-coding-openai::glm-5", chat_request.clone(), None).await {
		Ok(response) => {
			println!(
				"Response: {}",
				response.content.first_text().unwrap_or("No text response")
			);
		}
		Err(e) => {
			println!("Error (might need API key setup): {}", e);
		}
	}

	// Example 2: Using deepseek-v3.2 models (chat)
	println!("=== Example 1: Using deepseek-v3.2 ===");
	let chat_request = ChatRequest::default().append_message(ChatMessage::user("Hello, how are you?"));

	match client
		.exec_chat("baidu-coding-openai::deepseek-v3.2", chat_request.clone(), None)
		.await
	{
		Ok(response) => {
			println!(
				"Response: {}",
				response.content.first_text().unwrap_or("No text response")
			);
		}
		Err(e) => {
			println!("Error (might need API key setup): {}", e);
		}
	}

	// Example 2: Using minimax-m2.5 models (chat)
	println!("=== Example 1: Using minimax-m2.5 ===");
	let chat_request = ChatRequest::default().append_message(ChatMessage::user("Hello, how are you?"));

	match client
		.exec_chat("baidu-coding-openai::minimax-m2.5", chat_request.clone(), None)
		.await
	{
		Ok(response) => {
			println!(
				"Response: {}",
				response.content.first_text().unwrap_or("No text response")
			);
		}
		Err(e) => {
			println!("Error (might need API key setup): {}", e);
		}
	}

	Ok(())
}
