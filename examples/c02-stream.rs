use futures::StreamExt;
use genai::ollama::OllamaAdapter;
use genai::openai::OpenAIAdapter;
use genai::{ChatMessage, ChatRequest, ChatStream, Client};
use tokio::io::AsyncWriteExt as _;

const MODEL_OA: &str = "gpt-3.5-turbo";
const MODEL_OL: &str = "mixtral";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// -- Create the ChatReq
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Why is the sky red?")]);

	// -- Exec with OpenAI
	let api_key = std::env::var("OPENAI_API_KEY")?;
	let oa_client = OpenAIAdapter::client_from_api_key(api_key)?;
	let res = oa_client.exec_chat_stream(MODEL_OA, chat_req.clone()).await?;
	println!("=== RESPONSE from OpenAI ({MODEL_OA}):");
	print_gen_stream(res).await?;

	println!();

	// -- Exec with Ollama
	let oa_client = OllamaAdapter::default_client();
	let res = oa_client.exec_chat_stream(MODEL_OL, chat_req.clone()).await?;
	println!("=== RESPONSE from Ollama ({MODEL_OL}):");
	print_gen_stream(res).await?;

	Ok(())
}

pub async fn print_gen_stream(chat_res: ChatStream) -> Result<(), Box<dyn std::error::Error>> {
	let mut stdout = tokio::io::stdout();
	let mut char_count = 0;

	// let mut final_data_responses = Vec::new();
	let mut stream = chat_res.stream;

	while let Some(Ok(stream_item)) = stream.next().await {
		let Some(response) = stream_item.content else {
			stdout.write_all(b"\nEMPTY RESPONSE - CONTINUE\n").await?;
			continue;
		};

		let bytes = response.as_bytes();

		// Poor man's wrapping
		char_count += bytes.len();
		if char_count > 80 {
			stdout.write_all(b"\n").await?;
			char_count = 0;
		}

		// Write output
		stdout.write_all(bytes).await?;
		stdout.flush().await?;

		// if let Some(final_data) = res.final_data {
		// 	stdout.write_all(b"\n").await?;
		// 	stdout.flush().await?;
		// 	final_data_responses.push(final_data);
		// 	break;
		// }
	}

	stdout.write_all(b"\n").await?;
	stdout.flush().await?;

	Ok(())
}
