//! Base examples demonstrating the core capabilities of genai

use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

const MODEL_OPENAI: &str = "gpt-4o-mini"; // o1-mini, gpt-4o-mini
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
const MODEL_COHERE: &str = "command-light";
const MODEL_GEMINI: &str = "gemini-1.5-flash-latest";
const MODEL_GROQ: &str = "gemma-7b-it";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`

// NOTE: These are the default environment keys for each AI Adapter Type.
//       They can be customized; see `examples/c02-auth.rs`
const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	// -- De/activate models/providers
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_GEMINI, "GEMINI_API_KEY"),
	(MODEL_GROQ, "GROQ_API_KEY"),
	(MODEL_OLLAMA, ""),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - starts_with "gemini"   -> Gemini
//  - model in Groq models   -> Groq
//  - For anything else      -> Ollama
//
// This can be customized; see `examples/c03-kind.rs`

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	let print_options = PrintChatStreamOptions::from_print_events(false);

	for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
		// Skip if the environment name is not set
		if !env_name.is_empty() && std::env::var(env_name).is_err() {
			println!("===== Skipping model: {model} (env var not set: {env_name})");
			continue;
		}

		let adapter_kind = client.resolve_model_iden(model)?.adapter_kind;

		println!("\n===== MODEL: {model} ({adapter_kind}) =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer:");
		let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
		println!("{}", chat_res.content_text_as_str().unwrap_or("NO ANSWER"));

		println!("\n--- Answer: (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;
		print_chat_stream(chat_res, Some(&print_options)).await?;

		println!();
	}

	Ok(())
}
