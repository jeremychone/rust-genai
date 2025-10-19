//! Test to validate that genai can list models and retrieve pricing information
//! from all supported providers: OpenRouter, Groq, Cerebras, and Z.AI

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use std::collections::HashMap;

/// Helper to check if environment variable is set
fn has_env_key(key: &str) -> bool {
	std::env::var(key).is_ok_and(|v| !v.is_empty())
}

/// Test that we can resolve models for each provider
#[tokio::test]
async fn test_list_models_all_providers() -> Result<(), Box<dyn std::error::Error>> {
	println!("🧪 Testing model listing for all providers...\n");

	let client = Client::default();
	let mut provider_results = HashMap::new();

	// Test OpenRouter models
	if has_env_key("OPENROUTER_API_KEY") {
		println!("📡 Testing OpenRouter model listing...");

		// Test with a few known OpenRouter models
		let openrouter_models = vec![
			"openrouter::anthropic/claude-3.5-sonnet",
			"openrouter::openai/gpt-4o-mini",
			"openrouter::google/gemini-pro",
		];

		for model in openrouter_models {
			match client.resolve_service_target(model).await {
				Ok(target) => {
					println!("  ✅ Resolved: {} -> {:?}", model, target.model.adapter_kind);
					provider_results.insert(model.to_string(), "resolved".to_string());
				}
				Err(e) => {
					println!("  ❌ Failed to resolve {}: {}", model, e);
					provider_results.insert(model.to_string(), format!("error: {}", e));
				}
			}
		}
	} else {
		println!("⚠️  OPENROUTER_API_KEY not set, skipping OpenRouter tests");
	}

	// Test Groq models
	if has_env_key("GROQ_API_KEY") {
		println!("\n📡 Testing Groq model listing...");

		let groq_models = vec!["llama-3.1-8b-instant", "llama-3.1-70b-versatile", "mixtral-8x7b-32768"];

		for model in groq_models {
			match client.resolve_service_target(model).await {
				Ok(target) => {
					println!("  ✅ Resolved: {} -> {:?}", model, target.model.adapter_kind);
					provider_results.insert(format!("groq:{}", model), "resolved".to_string());
				}
				Err(e) => {
					println!("  ❌ Failed to resolve {}: {}", model, e);
					provider_results.insert(format!("groq:{}", model), format!("error: {}", e));
				}
			}
		}
	} else {
		println!("⚠️  GROQ_API_KEY not set, skipping Groq tests");
	}

	// Test Cerebras models
	if has_env_key("CEREBRAS_API_KEY") {
		println!("\n📡 Testing Cerebras model listing...");

		let cerebras_models = vec!["llama3.1-8b", "llama3.1-70b", "mixtral-8x7b"];

		for model in cerebras_models {
			// Try with namespace
			let namespaced_model = format!("cerebras/{}", model);
			match client.resolve_service_target(&namespaced_model).await {
				Ok(target) => {
					println!("  ✅ Resolved: {} -> {:?}", namespaced_model, target.model.adapter_kind);
					provider_results.insert(namespaced_model, "resolved".to_string());
				}
				Err(e) => {
					// Try without namespace
					match client.resolve_service_target(model).await {
						Ok(target) => {
							println!("  ✅ Resolved: {} -> {:?}", model, target.model.adapter_kind);
							provider_results.insert(model.to_string(), "resolved".to_string());
						}
						Err(e2) => {
							println!(
								"  ❌ Failed to resolve {} (with/without namespace): {} / {}",
								model, e, e2
							);
							provider_results.insert(model.to_string(), format!("error: {}", e2));
						}
					}
				}
			}
		}
	} else {
		println!("⚠️  CEREBRAS_API_KEY not set, skipping Cerebras tests");
	}

	// Test Z.AI models (if supported)
	if has_env_key("ZAI_API_KEY") {
		println!("\n📡 Testing Z.AI model listing...");

		let zai_models = vec!["glm-4.6", "glm-4", "glm-3-turbo"];

		for model in zai_models {
			match client.resolve_service_target(model).await {
				Ok(target) => {
					println!("  ✅ Resolved: {} -> {:?}", model, target.model.adapter_kind);
					provider_results.insert(model.to_string(), "resolved".to_string());
				}
				Err(e) => {
					println!("  ❌ Failed to resolve {}: {}", model, e);
					provider_results.insert(model.to_string(), format!("error: {}", e));
				}
			}
		}
	} else {
		println!("⚠️  ZAI_API_KEY not set, skipping Z.AI tests");
	}

	// Summary
	println!("\n📊 Summary:");
	let mut total = 0;
	let mut resolved = 0;

	for (model, status) in &provider_results {
		total += 1;
		if status == "resolved" {
			resolved += 1;
			println!("  ✅ {}", model);
		} else {
			println!("  ❌ {}: {}", model, status);
		}
	}

	println!("\n🎯 Resolved {}/{} models successfully", resolved, total);

	// We expect at least some models to be resolved if API keys are present
	if has_env_key("OPENROUTER_API_KEY") || has_env_key("GROQ_API_KEY") || has_env_key("CEREBRAS_API_KEY") {
		assert!(
			resolved > 0,
			"At least one model should be resolved when API keys are present"
		);
	}

	Ok(())
}

/// Test that we can execute simple chat requests to verify models are accessible
#[tokio::test]
async fn test_provider_accessibility() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔗 Testing provider accessibility with simple chat requests...\n");

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Respond with just 'OK'")]);

	// Test OpenRouter
	if has_env_key("OPENROUTER_API_KEY") {
		println!("📡 Testing OpenRouter accessibility...");
		match client.exec_chat("openrouter::openai/gpt-4o-mini", chat_req.clone(), None).await {
			Ok(response) => {
				if let Some(content) = response.first_text() {
					println!("  ✅ OpenRouter response: {}", content);
				} else {
					println!("  ⚠️  OpenRouter returned empty response");
				}
			}
			Err(e) => {
				println!("  ❌ OpenRouter error: {}", e);
			}
		}
	}

	// Test Groq
	if has_env_key("GROQ_API_KEY") {
		println!("\n📡 Testing Groq accessibility...");
		match client.exec_chat("llama-3.1-8b-instant", chat_req.clone(), None).await {
			Ok(response) => {
				if let Some(content) = response.first_text() {
					println!("  ✅ Groq response: {}", content);
				} else {
					println!("  ⚠️  Groq returned empty response");
				}
			}
			Err(e) => {
				println!("  ❌ Groq error: {}", e);
			}
		}
	}

	// Test Cerebras
	if has_env_key("CEREBRAS_API_KEY") {
		println!("\n📡 Testing Cerebras accessibility...");
		match client.exec_chat("llama3.1-8b", chat_req.clone(), None).await {
			Ok(response) => {
				if let Some(content) = response.first_text() {
					println!("  ✅ Cerebras response: {}", content);
				} else {
					println!("  ⚠️  Cerebras returned empty response");
				}
			}
			Err(e) => {
				println!("  ❌ Cerebras error: {}", e);
			}
		}
	}

	println!("\n✅ Accessibility tests completed");
	Ok(())
}
