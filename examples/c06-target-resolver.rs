//! This example demonstrates how to use a custom ServiceTargetResolver which gives full control over the final
//! mapping for Endpoint, Model/AdapterKind, and Auth.
//!
//! IMPORTANT - Here we are using xAI as an example of a custom ServiceTarget.
//!             However, there is now an XaiAdapter, which gets activated on `starts_with("grok")`.

use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};

const MODEL: &str = "grok-beta";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

	let questions = &[
		// Follow-up questions
		"Why is the sky blue?",
		"Why is it red sometimes?",
	];

	// -- Build an auth_resolver and the AdapterConfig
	let target_resolver = ServiceTargetResolver::from_resolver_fn(
		|service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
			let ServiceTarget { model, .. } = service_target;
			let endpoint = Endpoint::from_static("https://api.x.ai/v1/");
			let auth = AuthData::from_env("XAI_API_KEY");
			let model = ModelIden::new(AdapterKind::OpenAI, model.model_name);
			// TODO: point to xai
			Ok(ServiceTarget { endpoint, auth, model })
		},
	);

	// -- Build the new client with this adapter_config
	let client = Client::builder().with_service_target_resolver(target_resolver).build();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;

		println!("\n--- Answer: ");
		let assistant_answer = chat_res.content_text_as_str().ok_or("Should have response")?;
		println!("{assistant_answer}");

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
