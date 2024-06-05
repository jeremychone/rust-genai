mod support; // For examples support funtions
use crate::support::{has_env, print_chat_stream};

use genai::chat::{ChatMessage, ChatRequest};
use genai::client::Client;

const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
const MODEL_OPENAI: &str = "gpt-3.5-turbo";
const MODEL_OLLAMA: &str = "mixtral";

const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	(MODEL_OLLAMA, ""),
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
];

// NOTE: for now, Client Adapter/Provider mapping rule
//  - starts_with "claude" -> Anthropic
//  - starts_with "gpt"    -> OpenAI
//  - For anything else    -> Ollama
// Refined mapping rules will be added later and extended as provider support grows.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::new()?;

	for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
		// Skip if does not have the environment name set
		if !env_name.is_empty() && !has_env(env_name) {
			continue;
		}

		println!("\n===== MODEL: {model} =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer: (oneshot response)");
		let chat_res = client.exec_chat(model, chat_req.clone()).await?;
		println!("{}", chat_res.content.as_deref().unwrap_or("NO ANSWER"));

		println!("\n--- Answer: (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone()).await?;
		print_chat_stream(chat_res).await?;

		println!();
	}

	Ok(())
}
