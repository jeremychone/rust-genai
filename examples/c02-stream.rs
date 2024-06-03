mod support;
use support::print_chat_stream;

use genai::ollama::OllamaAdapter;
use genai::openai::OpenAIAdapter;
use genai::{ChatMessage, ChatRequest, Client};

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	// -- Exec with OpenAI
	let api_key = std::env::var("OPENAI_API_KEY")?;
	let oa_client = OpenAIAdapter::client_from_api_key(api_key)?;
	let res = oa_client.exec_chat_stream(MODEL_OA, chat_req.clone()).await?;
	println!("\n=== QUESTION: {question}\n");
	println!("=== RESPONSE from OpenAI ({MODEL_OA}):\n");
	print_chat_stream(res).await?;

	// -- Exec with Ollama
	let oa_client = OllamaAdapter::default_client();
	let res = oa_client.exec_chat_stream(MODEL_OL, chat_req.clone()).await?;
	println!("\n=== QUESTION: {question}\n");
	println!("=== RESPONSE from Ollama ({MODEL_OL}):\n");
	print_chat_stream(res).await?;

	Ok(())
}
