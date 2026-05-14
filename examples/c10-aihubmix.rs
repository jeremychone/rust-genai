//! AIHubMix adapter example
//!
//! Demonstrates how to use AIHubMix models with namespace-based access:
//! - `aihubmix::gpt-4o-mini` → Routes to AIHubMix adapter (OpenAI-compatible)
//!
//! Required environment variable: AIHUBMIX_API_KEY

use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatRequest};

const MODEL: &str = "aihubmix::gpt-4o-mini";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let chat_req = ChatRequest::default()
		.with_system("You are a helpful assistant.")
		.append_message(ChatMessage::user("Say 'hello' and nothing else."));

	println!("\n=== AIHubMix Chat ===");
	println!("Model: {MODEL}");

	match client.exec_chat(MODEL, chat_req.clone(), None).await {
		Ok(response) => {
			println!("✅ Chat response:");
			if let Some(content) = response.first_text() {
				println!("  {}", content);
			}
		}
		Err(e) => {
			println!("❌ Error: {e}");
			if e.to_string().contains("401") {
				println!("ℹ️  Set AIHUBMIX_API_KEY environment variable");
			}
		}
	}

	println!("\n=== AIHubMix Stream ===");

	match client.exec_chat_stream(MODEL, chat_req, None).await {
		Ok(stream) => {
			println!("✅ Stream response:");
			let print_options = PrintChatStreamOptions::from_print_events(false);
			print_chat_stream(stream, Some(&print_options)).await?;
		}
		Err(e) => {
			println!("❌ Error: {e}");
		}
	}

	println!("\n=== Summary ===");
	println!("✅ AIHubMix adapter uses namespace routing (aihubmix::)");
	println!("✅ Set AIHUBMIX_API_KEY environment variable");

	Ok(())
}
