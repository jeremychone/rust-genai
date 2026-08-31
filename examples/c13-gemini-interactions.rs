//! Demonstrate the Gemini Interactions API adapter: server-side conversation state.
//!
//! Two things to note:
//!   1. `gemini-3*` models resolve to this adapter automatically. Earlier Gemini models stay on
//!      `generateContent`. Force either with `gemini_interactions::` / `gemini::`.
//!   2. `store` defaults to `true`, following the API. That means the conversation is retained
//!      server-side (55 days on the paid tier, 1 day on the free tier) — which is what makes
//!      `previous_response_id` resolvable. Pass `store: Some(false)` to opt out; that also
//!      disables continuation.
//!
//! Requires: GEMINI_API_KEY environment variable.
//!
//! Run: `GEMINI_API_KEY=... cargo run --example c13-gemini-interactions`

use genai::Client;
use genai::chat::ChatRequest;

const MODEL: &str = "gemini-3.5-flash-lite";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::new()?;

	// -- Turn 1: open the conversation. `store` defaults to true, so the server remembers it.
	let chat_req = ChatRequest::from_user("My favorite language is Rust. Reply with just 'noted'.");

	let res_1 = client.exec_chat(MODEL, chat_req, None).await?;
	println!("--- Turn 1\n{}\n", res_1.first_text().unwrap_or("NO ANSWER"));

	let interaction_id = res_1.response_id.clone().ok_or("Expected an interaction id")?;
	println!("interaction id: {interaction_id}\n");

	// -- Turn 2: only the new message travels. The server supplies the history.
	//
	// NOTE: `system`, `tools` and the generation options are interaction-scoped — the server does
	//       NOT carry them across `previous_interaction_id`, so re-send them every turn if you
	//       want them to apply.
	let chat_req = ChatRequest::from_user("What is my favorite language?").with_previous_response_id(&interaction_id);

	let res_2 = client.exec_chat(MODEL, chat_req, None).await?;
	println!("--- Turn 2\n{}\n", res_2.first_text().unwrap_or("NO ANSWER"));

	let cached_tokens = res_2
		.usage
		.prompt_tokens_details
		.as_ref()
		.and_then(|details| details.cached_tokens);
	println!("--- Usage");
	println!("prompt_tokens: {:?}", res_2.usage.prompt_tokens);
	println!("cached_tokens: {cached_tokens:?}");

	Ok(())
}
