use genai::ollama::OllamaAdapter;
use genai::openai::OpenAIAdapter;
use genai::{ChatMessage, ChatRequest, Client};

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Why is the sky red?")]);

	// -- Exec with OpenAI
	let api_key = std::env::var("OPENAI_API_KEY")?;
	let oa_client = OpenAIAdapter::client_from_api_key(api_key)?;
	let res = oa_client.exec_chat(MODEL_OA, chat_req.clone()).await?;
	println!(
		"=== RESPONSE from OpenAI ({MODEL_OA}):\n{}",
		res.content.as_deref().unwrap_or("NO ANSWER")
	);

	println!("\n");

	// -- Exec with Ollama
	let ol_client = OllamaAdapter::default_client();
	let res = ol_client.exec_chat(MODEL_OL, chat_req.clone()).await?;
	println!(
		"=== RESPONSE from Ollama ({MODEL_OL}):\n{}",
		res.content.as_deref().unwrap_or("NO ANSWER")
	);

	Ok(())
}
