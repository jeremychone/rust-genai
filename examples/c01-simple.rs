mod support;

use crate::support::has_env;
use genai::anthropic::AnthropicProvider;
use genai::ext_async_openai::OpenAIProvider;
use genai::ext_ollama_rs::OllamaProvider;
use genai::{ChatMessage, ChatRequest, LegacyClient, LegacyClientConfig};

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";
const MODEL_AN: &str = "claude-3-haiku-20240307";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![
		// -- Messages (activate to see the differences)
		// ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	// -- Exec with Ollama
	println!("\n=== QUESTION: {question}\n");
	let ol_client = OllamaProvider::default_client();
	let res = ol_client.exec_chat(MODEL_OL, chat_req.clone()).await?;
	println!(
		"=== RESPONSE from Ollama ({MODEL_OL}):\n\n{}",
		res.content.as_deref().unwrap_or("NO ANSWER")
	);

	println!();

	// -- Exec with OpenAI
	if has_env("OPENAI_API_KEY") {
		let config = LegacyClientConfig::from_key(std::env::var("OPENAI_API_KEY")?);
		let oa_client = OpenAIProvider::new_client(config)?;
		let res = oa_client.exec_chat(MODEL_OA, chat_req.clone()).await?;
		println!("\n=== QUESTION: {question}\n");
		println!(
			"=== RESPONSE from OpenAI ({MODEL_OA}):\n\n{}",
			res.content.as_deref().unwrap_or("NO ANSWER")
		);
	}

	println!();

	// -- Exec with Anthropic
	if has_env(AnthropicProvider::DEFAULT_API_KEY_ENV_NAME) {
		println!("\n=== QUESTION: {question}\n");

		let an_client = AnthropicProvider::default_client();
		let res = an_client.exec_chat(MODEL_AN, chat_req.clone()).await?;
		println!(
			"=== RESPONSE from Anthropic ({MODEL_AN}):\n\n{}",
			res.content.as_deref().unwrap_or("NO ANSWER")
		);
	}

	Ok(())
}
