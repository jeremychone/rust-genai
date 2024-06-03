mod support;
use support::print_chat_stream;

use genai::ollama::OllamaAdapter;
use genai::{ChatMessage, ChatRequest, Client};

const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let questions = &[
		// user list of questions
		"Why is the sky red?",
		"Why does people say it is blue?", // here it is to show conversation is passed back
	];

	let oa_client = OllamaAdapter::default_client();

	let mut last_model_response: Option<String> = None;
	let mut chat_req = ChatRequest::new(Vec::new());

	for &question in questions {
		println!("\n=== QUESTION: {question}\n");
		if let Some(last_response) = last_model_response.take() {
			chat_req.append_message(ChatMessage::assistant(last_response));
		}
		chat_req.append_message(ChatMessage::user(question));

		let chat_stream = oa_client.exec_chat_stream(MODEL_OL, chat_req.clone()).await?;

		println!("=== RESPONSE from Ollama ({MODEL_OL}):\n");
		let model_response = print_chat_stream(chat_stream).await?;

		last_model_response = Some(model_response);
		println!()
	}

	Ok(())
}
