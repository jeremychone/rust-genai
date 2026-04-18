//! GitHub Copilot OAuth device flow example.
//!
//! This example prompts you through the device flow, persists refreshed tokens automatically,
//! and then uses the Copilot service target resolver to run a chat request.

use genai::Client;
use genai::adapter::{CopilotTokenManager, PrintCopilotCallback};
use genai::chat::{ChatMessage, ChatRequest};

const MODEL: &str = "github_copilot::gpt-4o";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let token_manager = CopilotTokenManager::new(PrintCopilotCallback);
	let resolver = token_manager.into_service_target_resolver();

	let client = Client::builder().with_service_target_resolver(resolver).build();

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	]);

	println!("\n--- Chat Response:");
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;
	let answer = chat_res.first_text().ok_or_else(|| std::io::Error::other("NO ANSWER"))?;
	println!("{answer}");

	Ok(())
}
