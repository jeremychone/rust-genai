use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, AuthResolver};
use genai::{Client, ModelIden};

const MODEL: &str = "gpt-4o-mini";

/// This example demonstrates how to use a custom authentication function to override the default AuthData resolution
/// for any specific adapter (which is based on environment variables).

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let questions = &[
		// follow-up questions
		"Why is the sky blue?",
		"Why is it red sometime?",
	];

	// -- Build a auth_resolver and the AdapterConfig
	let auth_resolver = AuthResolver::from_resolver_fn(
		|model_iden: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
			let ModelIden {
				adapter_kind,
				model_name,
			} = model_iden;
			println!("\n>> Custom auth provider for {adapter_kind} (model: {model_name}) <<");

			// This will cause it to fail if any model is not an OPEN_API_KEY
			let key = std::env::var("OPENAI_API_KEY").map_err(|_| genai::resolver::Error::ApiKeyEnvNotFound {
				env_name: "OPENAI_API_KEY".to_string(),
			})?;
			Ok(Some(AuthData::from_single(key)))
		},
	);

	// -- Build the new client with this adapter_config
	let client = Client::builder().with_auth_resolver(auth_resolver).build();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

		println!("\n--- Answer: (streaming)");
		let assistant_answer = print_chat_stream(chat_res, None).await?;

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
