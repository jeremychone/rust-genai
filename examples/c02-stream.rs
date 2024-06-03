mod support;
use support::print_chat_stream;

use genai::ollama::OllamaProvider;
use genai::openai::OpenAIProvider;
use genai::{ChatMessage, ChatRequest, Client};

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	// -- Exec with OpenAI
	let oa_client = OpenAIProvider::default_client(); // will use default env key
	let res = oa_client.exec_chat_stream(MODEL_OA, chat_req.clone()).await?;
	println!("\n=== QUESTION: {question}\n");
	println!("=== RESPONSE from OpenAI ({MODEL_OA}):\n");
	print_chat_stream(res).await?;

	// -- Exec with Ollama
	let oa_client = OllamaProvider::default_client();
	let res = oa_client.exec_chat_stream(MODEL_OL, chat_req.clone()).await?;
	println!("\n=== QUESTION: {question}\n");
	println!("=== RESPONSE from Ollama ({MODEL_OL}):\n");
	print_chat_stream(res).await?;

	Ok(())
}
