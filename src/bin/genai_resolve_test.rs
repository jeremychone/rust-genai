use genai::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// Test model patterns and their resolution
	let test_models = vec![
		// OpenAI patterns
		("gpt-4o-mini", "OpenAI"),
		("gpt-4-turbo", "OpenAI"),
		("o1-preview", "OpenAI"),
		// Anthropic patterns
		("claude-3-5-sonnet-20241022", "Anthropic"),
		("claude-3-haiku-20240307", "Anthropic"),
		// Gemini patterns
		("gemini-2.0-flash", "Gemini"),
		("gemini-pro", "Gemini"),
		// Groq patterns (should resolve to Groq)
		("llama-3.1-8b-instant", "Groq"),
		("llama-3.1-70b-versatile", "Groq"),
		// Cohere patterns
		("command-r-plus", "Cohere"),
		("command-light", "Cohere"),
		// DeepSeek patterns
		("deepseek-chat", "DeepSeek"),
		("deepseek-coder", "DeepSeek"),
		// Namespaced models
		("openrouter::anthropic/claude-3.5-sonnet", "OpenRouter"),
		("cerebras::llama3.1-8b", "Cerebras"),
		("openai::gpt-4o", "OpenAI"),
		// Default (should go to Ollama)
		("codellama:7b", "Ollama"),
		("mistral", "Ollama"),
		// Z.AI models (GLM models)
		("glm-4.6", "ZAi"),
		("glm-4", "ZAi"),
	];

	println!("Model Resolution Test Results:");
	println!("=============================");
	println!();

	let mut success_count = 0;
	let mut total_count = 0;

	for (model, expected_provider) in test_models {
		total_count += 1;

		match client.resolve_service_target(model).await {
			Ok(target) => {
				let actual_provider = format!("{:?}", target.model.adapter_kind);
				if actual_provider.contains(expected_provider) {
					println!(
						"✅ {:<35} -> {} (expected: {})",
						model, actual_provider, expected_provider
					);
					success_count += 1;
				} else {
					println!(
						"⚠️  {:<35} -> {} (expected: {})",
						model, actual_provider, expected_provider
					);
				}
			}
			Err(e) => {
				println!("❌ {:<35} -> ERROR: {}", model, e);
			}
		}
	}

	println!();
	println!(
		"Summary: {}/{} models resolved successfully",
		success_count, total_count
	);

	Ok(())
}
