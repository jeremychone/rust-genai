use genai::ollama::OllamaProvider;
use genai::openai::OpenAIProvider;
use genai::{ChatMessage, ChatRequest, Client, ClientConfig};

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![ChatMessage::user(question)]);

	// -- Exec with OpenAI
	let config = ClientConfig::from_key(std::env::var("OPENAI_API_KEY")?);
	let oa_client = OpenAIProvider::new_client(config)?;
	let res = oa_client.exec_chat(MODEL_OA, chat_req.clone()).await?;
	println!("\n=== QUESTION: {question}\n");
	println!(
		"=== RESPONSE from OpenAI ({MODEL_OA}):\n\n{}",
		res.content.as_deref().unwrap_or("NO ANSWER")
	);

	// -- Exec with Ollama
	let ol_client = OllamaProvider::default_client();
	let res = ol_client.exec_chat(MODEL_OL, chat_req.clone()).await?;
	println!("\n=== QUESTION: {question}\n");
	println!(
		"=== RESPONSE from Ollama ({MODEL_OL}):\n\n{}",
		res.content.as_deref().unwrap_or("NO ANSWER")
	);

	Ok(())
}
