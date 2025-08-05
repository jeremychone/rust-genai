//! This example demonstrates how to properly attach image to the conversations

use genai::Client;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest, ContentPart};
use tracing_subscriber::EnvFilter;

const MODEL: &str = "gpt-4o-mini";
const IMAGE_URL: &str = "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		// .with_max_level(tracing::Level::DEBUG) // To enable all sub-library tracing
		.init();

	let client = Client::default();

	let question = "What is in this picture?";

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text(question),
		ContentPart::from_binary_url(None, "image/jpg", IMAGE_URL),
	]));

	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

	println!("\n--- Answer: (streaming)");
	let _assistant_answer = print_chat_stream(chat_res, None).await?;

	Ok(())
}
