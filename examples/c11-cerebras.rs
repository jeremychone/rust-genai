//! Cerebras basic chat and streaming example

use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatRequest};

const MODEL_CEREBRAS: &str = "cerebras::llama-3.1-8b";
const CEREBRAS_ENV: &str = "CEREBRAS_API_KEY";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	if std::env::var(CEREBRAS_ENV).is_err() {
		println!(
			"Skipping: set {} to run this example (e.g., export {}=...)",
			CEREBRAS_ENV, CEREBRAS_ENV
		);
		return Ok(());
	}

	let question = "Why do stars twinkle?";

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer briefly in one sentence."),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	println!("\n--- MODEL: {}", MODEL_CEREBRAS);
	println!("\n--- Question:\n{}", question);

	println!("\n--- Answer:");
	let chat_res = client.exec_chat(MODEL_CEREBRAS, chat_req.clone(), None).await?;
	println!("{}", chat_res.first_text().unwrap_or("NO ANSWER"));

	println!("\n--- Answer (streaming):");
	let stream = client.exec_chat_stream(MODEL_CEREBRAS, chat_req, None).await?;
	let print_options = PrintChatStreamOptions::from_print_events(false);
	print_chat_stream(stream, Some(&print_options)).await?;

	Ok(())
}
