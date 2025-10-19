//! Test to verify the model resolution fixes

use genai::Client;

#[tokio::test]
async fn test_model_resolution_fixes() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// Test deepseek-coder now resolves to DeepSeek
	let target = client.resolve_service_target("deepseek-coder").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "DeepSeek");
	println!("✅ deepseek-coder -> DeepSeek");

	// Test cerebras::llama3.1-8b resolves to Cerebras
	let target = client.resolve_service_target("cerebras::llama3.1-8b").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "Cerebras");
	println!("✅ cerebras::llama3.1-8b -> Cerebras");

	// Test openai::gpt-4o resolves to OpenAI (not OpenRouter)
	let target = client.resolve_service_target("openai::gpt-4o").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "OpenAI");
	println!("✅ openai::gpt-4o -> OpenAI");

	// Test that OpenRouter still works with non-namespaced models
	let target = client.resolve_service_target("openrouter::anthropic/claude-3.5-sonnet").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "OpenRouter");
	println!("✅ openrouter::anthropic/claude-3.5-sonnet -> OpenRouter");

	// Test that OpenRouter still catches non-namespaced / patterns
	let target = client.resolve_service_target("openai/gpt-4o-mini").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "OpenRouter");
	println!("✅ openai/gpt-4o-mini (no namespace) -> OpenRouter");

	println!("\n✨ All model resolution fixes verified!");
	Ok(())
}
