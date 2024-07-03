use genai::chat::{ChatMessage, ChatRequest, ChatRequestOptions};
use genai::client::Client;
use genai::utils::print_chat_stream;

// const MODEL: &str = "gpt-3.5-turbo";
const MODEL: &str = "mixtral";

/// This example shows how to use a custom AdapterKindResolver to have some custom
/// mapping from a model name to a AdapterKind.
/// This allows to map missing models to their Adapter implementations.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Build the new client with this client_config
	let client = Client::builder().build();

	// -- Build the chat request
	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	// -- Build the chat request options
	let options = ChatRequestOptions::default().with_temperature(0.0).with_max_tokens(1000);

	// -- Exec and print
	println!("\n--- Question:\n{question}");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), Some(&options)).await?;

	let adapter_kind = client.resolve_adapter_kind(MODEL)?;
	println!("\n--- Answer: ({adapter_kind})");
	print_chat_stream(chat_res, None).await?;

	Ok(())
}
