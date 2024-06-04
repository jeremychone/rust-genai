mod support;
use support::print_chat_stream;

use crate::support::has_env;
use genai::anthropic::AnthropicProvider;
use genai::ext_async_openai::OpenAIProvider;
use genai::ext_ollama_rs::OllamaProvider;
use genai::{ChatMessage, ChatRequest, Client};

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";
const MODEL_AN: &str = "claude-3-haiku-20240307";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![
		// Messages (activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	// -- Exec with Ollama
	println!("\n=== QUESTION: {question}\n");
	let oa_client = OllamaProvider::default_client();
	let res = oa_client.exec_chat_stream(MODEL_OL, chat_req.clone()).await?;
	println!("=== RESPONSE from Ollama ({MODEL_OL}):\n");
	print_chat_stream(res).await?;

	println!();

	// -- Exec with OpenAI
	if has_env(OpenAIProvider::DEFAULT_API_KEY_ENV_NAME) {
		println!("\n=== QUESTION: {question}\n");
		let oa_client = OpenAIProvider::default_client(); // will use default env key
		let res = oa_client.exec_chat_stream(MODEL_OA, chat_req.clone()).await?;
		println!("=== RESPONSE from OpenAI ({MODEL_OA}):\n");
		print_chat_stream(res).await?;
	}

	println!();

	// -- Exec with Anthropic
	if has_env(AnthropicProvider::DEFAULT_API_KEY_ENV_NAME) {
		println!("\n=== QUESTION: {question}\n");
		let an_client = AnthropicProvider::default_client();
		let res = an_client.exec_chat_stream(MODEL_AN, chat_req.clone()).await?;
		println!("=== RESPONSE from Anthropic ({MODEL_AN}):\n");
		print_chat_stream(res).await?;
	}

	Ok(())
}
