//! This example demonstrates how to properly attach image to the conversations

use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest, ContentPart, ImageSource};
use genai::Client;

const MODEL: &str = "gpt-4o-mini";
const IMAGE_URL: &str = "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	let question = "What is in this picture?";

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::Text(question.to_string()),
		ContentPart::Image {
			content: IMAGE_URL.to_string(),
			content_type: "image/jpg".to_string(),
			source: ImageSource::Url,
		},
	]));

	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

	println!("\n--- Answer: (streaming)");
	let _assistant_answer = print_chat_stream(chat_res, None).await?;

	Ok(())
}
