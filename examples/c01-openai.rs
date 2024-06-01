use genai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// create new client
	let api_key = std::env::var("OPENAI_API_KEY")?;
	let oa_client = OpenAIClient::from_api_key(api_key);

	//

	Ok(())
}
