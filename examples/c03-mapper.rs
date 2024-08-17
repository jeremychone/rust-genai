use genai::adapter::AdapterKind;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::ModelMapper;
use genai::{Client, ModelIden};

// NOTE: This will be overriden below to `gpt-4o-mini`
const MODEL: &str = "gpt-4o";

/// This example demonstrates how to use the ModelMapper to map a ModelIden (model identifier) to
/// a potentially different one, using the model mapper.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let questions = &[
		// follow-up questions
		"Why is the sky blue?",
		"Why is it red sometime?",
	];

	// -- Build a auth_resolver and the AdapterConfig
	let model_mapper = ModelMapper::from_mapper_fn(|model_iden: ModelIden| {
		// let's be cheap, and map all gpt to "gpt-4o-mini"
		if model_iden.model_name.starts_with("gpt-") {
			Ok(ModelIden::new(AdapterKind::OpenAI, "gpt-4o-mini"))
		} else {
			Ok(model_iden)
		}
	});

	// -- Build the new client with this client_config
	let client = Client::builder().with_model_mapper(model_mapper).build();

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

		println!(
			"\n--- Answer: ({} - {})",
			chat_res.model_iden.adapter_kind, chat_res.model_iden.model_name
		);
		let assistant_answer = print_chat_stream(chat_res, None).await?;

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
