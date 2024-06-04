mod support;

use crate::support::{has_env, print_chat_stream};
use genai::client::Client;
use genai::{ChatMessage, ChatRequest};

const MODEL_AN: &str = "claude-3-haiku-20240307";

const MODEL: &str = MODEL_AN;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![
		// -- Messages (activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	// -- Exec Chat
	println!("\n=== QUESTION: {question}\n");
	let client = Client::new()?;
	let chat_res = client.exec_chat(MODEL, chat_req.clone()).await?;
	println!(
		"=== RESPONSE  ({MODEL}):\n\n{}",
		chat_res.content.as_deref().unwrap_or("NO ANSWER")
	);

	// -- Exec Stream
	println!("\n=== QUESTION: {question}\n");
	let client = Client::new()?;
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone()).await?;
	println!("=== RESPONSE ({MODEL}):\n");
	print_chat_stream(chat_res).await?;

	Ok(())
}
