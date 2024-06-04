mod support;
use support::print_chat_stream;

use genai::ext_ollama_rs::OllamaProvider;
use genai::{ChatMessage, ChatRequest, LegacyClient};

const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let questions = &[
		// user list of questions
		"Why is the sky red?",
		"Why does people say it is blue?", // here it is to show conversation is passed back
	];

	let oa_client = OllamaProvider::default_client();

	let mut chat_req = ChatRequest::new(vec![ChatMessage::system("Be concise in your answers")]);

	for &question in questions {
		println!("\n=== QUESTION: {question}\n");

		chat_req.append_message(ChatMessage::user(question));

		let chat_stream = oa_client.exec_chat_stream(MODEL_OL, chat_req.clone()).await?;

		println!("=== RESPONSE from Ollama ({MODEL_OL}):\n");
		let model_response = print_chat_stream(chat_stream).await?;

		chat_req.append_message(ChatMessage::assistant(model_response));

		println!()
	}

	Ok(())
}
