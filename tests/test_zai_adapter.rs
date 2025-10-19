//! Test for Z.AI adapter support

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};

#[tokio::test]
async fn test_zai_model_resolution() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// Test that Z.AI models resolve correctly
	let zai_models = vec!["glm-4.6", "glm-4", "glm-3-turbo"];

	for model in zai_models {
		let target = client.resolve_service_target(model).await?;
		assert_eq!(format!("{:?}", target.model.adapter_kind), "ZAi");
		println!("✅ {} -> ZAi", model);
	}

	// Test that namespaced Z.AI works
	let target = client.resolve_service_target("zai::glm-4.6").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "ZAi");
	println!("✅ zai::glm-4.6 -> ZAi");

	// Test that other GLM models not in list go to Zhipu (not Ollama)
	let target = client.resolve_service_target("glm-2").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "Zhipu");
	println!("✅ glm-2 -> Zhipu (not in Z.AI list, goes to Zhipu instead)");

	println!("\n✨ Z.AI model resolution tests passed!");
	Ok(())
}

#[tokio::test]
async fn test_zai_adapter_integration() -> Result<(), Box<dyn std::error::Error>> {
	// Only run if API key is available
	if std::env::var("ZAI_API_KEY").is_err() {
		println!("⚠️  ZAI_API_KEY not set, skipping integration test");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'Hello from Z.AI!'")]);

	let result = client.exec_chat("glm-4", chat_req, None).await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	println!("✅ Z.AI response: {}", content);

	Ok(())
}
