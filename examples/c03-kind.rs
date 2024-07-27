use genai::adapter::AdapterKind;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::AdapterKindResolver;
use genai::{Client, ClientConfig};

const MODEL: &str = "gpt-4o-mini";

/// This example shows how to use a custom AdapterKindResolver to have some custom
/// mapping from a model name to a AdapterKind.
/// This allows to map missing models to their Adapter implementations.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let questions = &[
		// follow-up questions
		"Why is the sky blue?",
		"Why is it red sometime?",
	];

	// -- Build a auth_resolver and the AdapterConfig
	let adapter_kind = AdapterKindResolver::from_sync_resolver(|model: &str| -> genai::Result<Option<AdapterKind>> {
		// Still use the default mapping to not break anything.
		let adapter_kind = AdapterKind::from_model(model)?;
		println!("\n>> Custom adapter kind resolver for model: {model} (AdapterKind {adapter_kind}) <<");
		Ok(Some(adapter_kind))
	});

	let client_config = ClientConfig::default().with_adapter_kind_resolver(adapter_kind);

	// -- Build the new client with this client_config
	let client = Client::builder().with_config(client_config).build();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat_stream(Some(MODEL), chat_req.clone(), None).await?;

		println!("\n--- Answer: ({MODEL})");
		let assistant_answer = print_chat_stream(chat_res, None).await?;

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
