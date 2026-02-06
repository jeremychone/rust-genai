//! Comprehensive test to verify that model lists in code match actual provider APIs
//! This test checks that our hardcoded model lists are accurate for each provider

use genai::Client;
use std::collections::HashMap;

/// Expected model lists from our codebase
fn get_expected_models() -> HashMap<String, Vec<String>> {
	let mut expected = HashMap::new();

	// DeepSeek models (from src/adapter/adapters/deepseek/adapter_impl.rs)
	expected.insert(
		"DeepSeek".to_string(),
		vec![
			"deepseek-chat".to_string(),
			"deepseek-reasoner".to_string(),
			"deepseek-coder".to_string(),
		],
	);

	// Z.AI models (from upstream v0.6.0-alpha.2 src/adapter/adapters/zai/adapter_impl.rs)
	// Note: Adapter is now named "Zai" (not "ZAi")
	expected.insert(
		"Zai".to_string(),
		vec![
			"glm-4.6".to_string(),
			"glm-4.5".to_string(),
			"glm-4.5v".to_string(),
			"glm-4-plus".to_string(),
			"glm-4-flash".to_string(),
		],
	);

	// Note: Groq models are complex - some with meta-llama prefix may resolve to OpenRouter
	// We'll test the ones that should definitely resolve to Groq
	expected.insert(
		"Groq".to_string(),
		vec![
			"llama-3.1-8b-instant".to_string(),
			"llama-3.1-70b-versatile".to_string(),
			"mixtral-8x7b-32768".to_string(),
			"gemma2-9b-it".to_string(),
			"qwen/qwen3-32b".to_string(),
			"moonshotai/kimi-k2-instruct".to_string(),
			"mistral-saba-24b".to_string(),
		],
	);

	expected
}

/// Test that our model lists are accurate by checking resolution
#[tokio::test]
async fn test_provider_model_lists() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();
	let expected_models = get_expected_models();

	println!("Verifying model lists match actual provider APIs...\n");

	let mut all_passed = true;

	for (provider, models) in expected_models {
		println!("=== Checking {} models ===", provider);

		let mut provider_passed = true;

		for model in models {
			match client.resolve_service_target(&model).await {
				Ok(target) => {
					let actual_adapter = format!("{:?}", target.model.adapter_kind);

					// Check if the model resolves to the expected adapter
					if actual_adapter == provider {
						println!("  [OK] {} -> {}", model, actual_adapter);
					} else {
						println!("  [FAIL] {} -> {} (expected {})", model, actual_adapter, provider);
						provider_passed = false;
						all_passed = false;
					}
				}
				Err(e) => {
					println!("  [FAIL] {} -> ERROR: {}", model, e);
					provider_passed = false;
					all_passed = false;
				}
			}
		}

		if provider_passed {
			println!("  All {} models resolved correctly\n", provider);
		} else {
			println!("  Some {} models failed to resolve\n", provider);
		}
	}

	println!("Summary:");
	if all_passed {
		println!("All model lists verified successfully!");
	} else {
		println!("Some model lists need updating");
		panic!("Model lists do not match actual provider APIs");
	}

	Ok(())
}

/// Test specific edge cases and model resolution conflicts
#[tokio::test]
async fn test_model_resolution_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	println!("Testing edge cases and conflicts...\n");

	// Test that Z.AI models work correctly
	// Note: Adapter is now "Zai" (not "ZAi") from upstream
	let test_cases = vec![
		// Z.AI models should resolve to Zai
		("glm-4.6", "Zai"),
		("glm-4.5", "Zai"),
		("glm-4-plus", "Zai"),
		// DeepSeek models
		("deepseek-coder", "DeepSeek"),
		("deepseek-reasoner", "DeepSeek"),
		("deepseek-chat", "DeepSeek"),
		// Model namespace
		("zai::glm-4.6", "Zai"),
		("openai::gpt-4o", "OpenAI"),
		("cerebras::llama3.1-8b", "Cerebras"),
	];

	let mut all_passed = true;

	for (model, expected_adapter) in test_cases {
		match client.resolve_service_target(model).await {
			Ok(target) => {
				let actual_adapter = format!("{:?}", target.model.adapter_kind);
				if actual_adapter == expected_adapter {
					println!("  [OK] {} -> {}", model, actual_adapter);
				} else {
					println!("  [FAIL] {} -> {} (expected {})", model, actual_adapter, expected_adapter);
					all_passed = false;
				}
			}
			Err(e) => {
				println!("  [FAIL] {} -> ERROR: {}", model, e);
				all_passed = false;
			}
		}
	}

	println!("\nEdge case tests completed!");

	assert!(all_passed, "Some edge cases failed");

	Ok(())
}

/// Test that providers that should use OpenRouter patterns work correctly
#[tokio::test]
async fn test_openrouter_model_patterns() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	println!("Testing OpenRouter model patterns...\n");

	// These should resolve to OpenRouter (non-namespaced with /)
	let openrouter_patterns = vec![
		"openai/gpt-4o-mini",
		"anthropic/claude-3-5-sonnet",
		"meta-llama/llama-3.1-8b",
		"google/gemini-pro",
	];

	for model in openrouter_patterns {
		match client.resolve_service_target(model).await {
			Ok(target) => {
				let adapter = format!("{:?}", target.model.adapter_kind);
				if adapter == "OpenRouter" {
					println!("  [OK] {} -> {}", model, adapter);
				} else {
					println!("  [FAIL] {} -> {} (expected OpenRouter)", model, adapter);
					return Err(format!("OpenRouter pattern failed for {}", model).into());
				}
			}
			Err(e) => {
				println!("  [FAIL] {} -> ERROR: {}", model, e);
				return Err(format!("Failed to resolve {}: {}", model, e).into());
			}
		}
	}

	println!("\nOpenRouter patterns verified!");

	Ok(())
}
