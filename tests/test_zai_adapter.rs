//! Test for Z.AI adapter support (upstream v0.6.0-alpha.2)
//! Note: Adapter is now named "Zai" (not "ZAi")

use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

#[tokio::test]
async fn test_zai_model_resolution() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// Test that Z.AI models resolve correctly (models in upstream ZAI MODELS list)
	let zai_models = vec!["glm-4.6", "glm-4.5", "glm-4-plus", "glm-4-flash"];

	for model in zai_models {
		let target = client.resolve_service_target(model).await?;
		assert_eq!(format!("{:?}", target.model.adapter_kind), "Zai");
		println!("[OK] {} -> Zai", model);
	}

	// Test that namespaced Z.AI works
	let target = client.resolve_service_target("zai::glm-4.6").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "Zai");
	println!("[OK] zai::glm-4.6 -> Zai");

	// Test that GLM models starting with "glm" go to Zai (per upstream logic)
	// In upstream, any model starting with "glm" resolves to Zai adapter
	let target = client.resolve_service_target("glm-2").await?;
	assert_eq!(format!("{:?}", target.model.adapter_kind), "Zai");
	println!("[OK] glm-2 -> Zai (any glm-* goes to Zai in upstream)");

	println!("\nZ.AI model resolution tests passed!");
	Ok(())
}

#[tokio::test]
async fn test_zai_adapter_integration() -> Result<(), Box<dyn std::error::Error>> {
	// Only run if API key is available
	if std::env::var("ZAI_API_KEY").is_err() {
		println!("ZAI_API_KEY not set, skipping integration test");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'Hello from Z.AI!'")]);

	let result = client.exec_chat("glm-4-flash", chat_req, None).await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	println!("[OK] Z.AI response: {}", content);

	Ok(())
}
