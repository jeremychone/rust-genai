//! This example demonstrates how to properly attach image to the conversations

use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest, ContentPart, ImageSource};
use genai::Client;

const MODEL: &str = "gpt-4o-mini";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(
		vec![
			ContentPart::Text("What is in this picture?"),
			ContentPart::Image {
				content: "IMAGE ENCODE BASE64".to_string(),
				content_type: "image/png".to_string(),
				source: ImageSource::Base64,
			}
		]
	));

	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

	println!("\n--- Answer: (streaming)");
	let assistant_answer = print_chat_stream(chat_res, None).await?;

	Ok(())
}
