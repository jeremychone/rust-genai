use genai::Client;
use genai::chat::printer::{PrintChatStreamOptions, print_chat_stream};
use genai::chat::{ChatMessage, ChatRequest};

// GitHub Copilot uses the GitHub Models API.
// Models are namespaced as `github_copilot::publisher/model-name`.
// Requires GITHUB_TOKEN environment variable (PAT with `models` scope).
const MODEL: &str = "github_copilot::openai/gpt-4.1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky blue?";

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	let adapter_kind = client.resolve_service_target(MODEL).await?.model.adapter_kind;

	println!("\n===== MODEL: {MODEL} ({adapter_kind}) =====");

	println!("\n--- Question:\n{question}");

	println!("\n--- Answer:");
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;
	println!("{}", chat_res.first_text().unwrap_or("NO ANSWER"));

	println!("\n--- Answer: (streaming)");
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;
	let print_options = PrintChatStreamOptions::from_print_events(false);
	print_chat_stream(chat_res, Some(&print_options)).await?;

	println!();

	Ok(())
}
