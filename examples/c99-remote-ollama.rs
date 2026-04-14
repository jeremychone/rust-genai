//! This example demonstrates how to use a custom ServiceTargetResolver to
//! point the genai Client to a remote (non-localhost) Ollama instance.

use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use tracing_subscriber::EnvFilter;

// Use a model you have pulled on your remote Ollama server
const MODEL: &str = "gpt-oss:120b";
const REMOTE_URL: &str = "http://XXX.XX.XX.XX:11434/"; // Change this to the address of your remote
// instance.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt().with_env_filter(EnvFilter::new("genai=debug")).init();

	let questions = &[
		// Follow-up questions
		"Why is the sky blue?",
		"Why is it red sometimes?",
	];

	// -- Build a target_resolver for Remote Ollama
	let target_resolver = ServiceTargetResolver::from_resolver_fn(
		|service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
			// Destructure to keep the original auth (Ollama usually has no auth)
			let ServiceTarget { auth, model, .. } = service_target;

			// 1. Define your custom remote endpoint
			// Make sure to include the /api/ suffix which Ollama uses
			let endpoint = Endpoint::from_static(REMOTE_URL);

			// 2. Force the client to use the Ollama Adapter
			let model = ModelIden::new(AdapterKind::Ollama, model.model_name);

			Ok(ServiceTarget { endpoint, auth, model })
		},
	);

	// -- Build the new client with this target_resolver
	let client = Client::builder().with_service_target_resolver(target_resolver).build();
	let service_target = ServiceTarget {
		endpoint: Endpoint::from_static(REMOTE_URL),
		auth: genai::resolver::AuthData::None,
		model: ModelIden::new(AdapterKind::Ollama, MODEL),
	};
	let all_models = client.all_model_names(AdapterKind::Ollama, Some(service_target)).await?;
	println!("Found models: {all_models:?}");

	// -- Normalize the eventual reasoning content
	let chat_options = ChatOptions::default().with_normalize_reasoning_content(true);

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat(MODEL, chat_req.clone(), Some(&chat_options)).await?;

		if let Some(reasoning_content) = chat_res.reasoning_content.as_deref() {
			println!("\n--- Reasoning:\n{reasoning_content}")
		}

		println!("\n--- Answer: ");
		let assistant_answer = chat_res.first_text().ok_or("Should have response")?;
		println!("{assistant_answer}");

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
