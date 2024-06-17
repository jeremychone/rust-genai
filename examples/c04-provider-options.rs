use genai::chat::{ChatMessage, ChatRequest, ChatRequestOptions, OpenAIOptions, ProviderOptions};
use genai::client::Client;
use genai::utils::print_chat_stream;

const MODEL: &str = "gpt-3.5-turbo";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let questions = &[
		// follow-up questions
		"Why is the sky blue?",
		"Why is it red sometime?",
	];

	let client = Client::default();

	// OpenAI options
	let openai_opts = OpenAIOptions {
		temperature: Some(0.2),
		max_tokens: Some(50),
		..Default::default()
	};

	let req_opts = ChatRequestOptions {
		provider_options: Some(ProviderOptions::OpenAI(openai_opts)),
		..Default::default()
	};

	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// Similar to put a first System Chat Message(s) (will be cummulative with sytem chat messages)

	for &question in questions {
		chat_req = chat_req.append_message(ChatMessage::user(question));

		println!("\n--- Question:\n{question}");
		let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), Some(&req_opts)).await?;

		println!("\n--- Answer: (streaming)");
		let assistant_answer = print_chat_stream(chat_res, None).await?;

		chat_req = chat_req.append_message(ChatMessage::assistant(assistant_answer));
	}

	Ok(())
}
