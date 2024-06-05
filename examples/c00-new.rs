mod support;

use crate::support::print_chat_stream;
use genai::client::Client;
use genai::{ChatMessage, ChatRequest};

const MODEL_AN: &str = "claude-3-haiku-20240307";
const MODEL_OA: &str = "gpt-3.5-turbo";

const MODELS: &[&str] = &[MODEL_AN, MODEL_OA];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		// ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::new()?;

	for model in MODELS {
		println!("\n===== MODEL: {model} =====");

		println!("\n--- Question: {question}");

		println!("\n--- Answer:   (oneshot response)");
		let chat_res = client.exec_chat(model, chat_req.clone()).await?;
		println!("{}", chat_res.content.as_deref().unwrap_or("NO ANSWER"));

		println!("\n--- Answer:   (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone()).await?;
		print_chat_stream(chat_res).await?;

		println!();
	}

	Ok(())
}
