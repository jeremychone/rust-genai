//! This example shows how to use a custom AdapterKindResolver to have some custom
//! mapping from a model name to a AdapterKind.
//! This allows to map missing models to their Adapter implementations.

use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::{Client, ClientConfig};

// const MODEL: &str = "gpt-4o-mini";
// const MODEL: &str = "command-light";
// const MODEL: &str = "claude-3-haiku-20240307";
// const MODEL: &str = "gemini-1.5-flash-latest";
// const MODEL: &str = "llama3-8b-8192";
const MODEL: &str = "gemma:2b";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Global ChatOptions
	// Note: The ChatOptions properties set at the client config level will be
	//       the fallback value if not provided at the chat exec level.
	let client_config =
		ClientConfig::default().with_chat_options(ChatOptions::default().with_temperature(0.0).with_top_p(0.99));

	// -- Build the new client with this client_config
	let client = Client::builder().with_config(client_config).build();

	// -- Build the chat request
	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	// -- Build the chat request options (used per exec chat)
	let options = ChatOptions::default().with_max_tokens(1000);

	// -- Exec and print
	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), Some(&options)).await?;

	let adapter_kind = client.resolve_model_iden(MODEL)?.adapter_kind;
	println!("\n--- Answer: ({MODEL} - {adapter_kind})");
	print_chat_stream(chat_res, None).await?;

	Ok(())
}
