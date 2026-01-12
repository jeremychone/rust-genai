//! Test to verify that our hardcoded model lists match the actual adapter code
//! This test ensures consistency between our test expectations and the actual codebase

use std::collections::HashMap;

/// Get actual model lists from adapter source files
fn read_adapter_models() -> HashMap<String, Vec<String>> {
	let mut models = HashMap::new();

	// Read from actual adapter files
	let base_path = std::env::current_dir().unwrap();

	// Helper function to extract models from a const array
	fn extract_model_array(content: &str, start_marker: &str) -> Option<Vec<String>> {
		let start = content.find(start_marker)?;
		let array_start = start + start_marker.len();

		// Find the closing ]; that matches the opening [
		let mut depth = 0;
		let mut end_pos = array_start;

		for (i, ch) in content[array_start..].char_indices() {
			match ch {
				'[' => depth += 1,
				']' => {
					if depth == 0 {
						end_pos = array_start + i;
						break;
					}
					depth -= 1;
				}
				_ => {}
			}
		}

		let models_str = &content[array_start..end_pos];

		// Extract quoted strings, ignoring comments
		let mut model_list = Vec::new();
		for line in models_str.lines() {
			let cleaned = line.split("//").next().unwrap_or(line); // Remove comments
			for part in cleaned.split(',') {
				let trimmed = part.trim();
				if let Some(model) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
					&& !model.is_empty()
				{
					model_list.push(model.to_string());
				}
			}
		}

		Some(model_list)
	}

	// DeepSeek models
	if let Ok(content) = std::fs::read_to_string(base_path.join("src/adapter/adapters/deepseek/adapter_impl.rs"))
		&& let Some(model_list) = extract_model_array(&content, "pub(in crate::adapter) const MODELS: &[&str] = &[")
	{
		models.insert("DeepSeek".to_string(), model_list);
	}

	// Z.AI models
	if let Ok(content) = std::fs::read_to_string(base_path.join("src/adapter/adapters/zai/adapter_impl.rs"))
		&& let Some(model_list) = extract_model_array(&content, "pub(in crate::adapter) const MODELS: &[&str] = &[")
	{
		models.insert("ZAi".to_string(), model_list);
	}

	// Groq models
	if let Ok(content) = std::fs::read_to_string(base_path.join("src/adapter/adapters/groq/adapter_impl.rs"))
		&& let Some(model_list) = extract_model_array(&content, "pub(in crate::adapter) const MODELS: &[&str] = &[")
	{
		models.insert("Groq".to_string(), model_list);
	}

	models
}

/// Expected model lists from our test expectations
fn get_expected_models() -> std::collections::HashMap<String, Vec<String>> {
	let mut expected = std::collections::HashMap::new();

	// DeepSeek models
	expected.insert(
		"DeepSeek".to_string(),
		vec![
			"deepseek-chat".to_string(),
			"deepseek-reasoner".to_string(),
			"deepseek-coder".to_string(),
		],
	);

	// Z.AI models
	expected.insert(
		"ZAi".to_string(),
		vec![
			"glm-4.6".to_string(),
			"glm-4.5".to_string(),
			"glm-4".to_string(),
			"glm-4.1v".to_string(),
			"glm-4.5v".to_string(),
			"vidu".to_string(),
			"vidu-q1".to_string(),
			"vidu-2.0".to_string(),
		],
	);

	// Groq models (all models from adapter code)
	expected.insert(
		"Groq".to_string(),
		vec![
			"moonshotai/kimi-k2-instruct".to_string(),
			"qwen/qwen3-32b".to_string(),
			"mistral-saba-24b".to_string(),
			"meta-llama/llama-4-scout-17b-16e-instruct".to_string(),
			"meta-llama/llama-4-maverick-17b-128e-instruct".to_string(),
			"llama-3.3-70b-versatile".to_string(),
			"llama-3.2-3b-preview".to_string(),
			"llama-3.2-1b-preview".to_string(),
			"llama-3.1-405b-reasoning".to_string(),
			"llama-3.1-70b-versatile".to_string(),
			"llama-3.1-8b-instant".to_string(),
			"mixtral-8x7b-32768".to_string(),
			"gemma2-9b-it".to_string(),
			"gemma-7b-it".to_string(),
			"llama-guard-3-8b".to_string(),
			"llama3-70b-8192".to_string(),
			"deepseek-r1-distill-llama-70b".to_string(),
			"llama-3.2-11b-vision-preview".to_string(),
			"llama-3.2-90b-vision-preview".to_string(),
		],
	);

	expected
}

/// Test that our test expectations match the actual adapter code
#[test]
fn test_adapter_code_consistency() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔍 Verifying test expectations match actual adapter code...\n");

	let actual_models = read_adapter_models();
	let test_models = get_expected_models();

	let mut all_consistent = true;

	for (provider, expected_list) in &test_models {
		println!("=== Checking {} ===", provider);

		match actual_models.get(provider) {
			Some(actual_list) => {
				// Check for differences
				let expected_set: std::collections::HashSet<_> = expected_list.iter().collect();
				let actual_set: std::collections::HashSet<_> = actual_list.iter().collect();

				if expected_set == actual_set {
					println!("  ✅ Test and code models match");
					println!("  📊 {} models", actual_set.len());
				} else {
					println!("  ❌ Mismatch found!");

					// Show differences
					let missing: Vec<_> = expected_set.difference(&actual_set).collect();
					let extra: Vec<_> = actual_set.difference(&expected_set).collect();

					if !missing.is_empty() {
						println!("  ⚠️  In test but not in code: {:?}", missing);
					}
					if !extra.is_empty() {
						println!("  ⚠️  In code but not in test: {:?}", extra);
					}
					all_consistent = false;
				}
			}
			None => {
				println!("  ❌ Provider {} not found in adapter code", provider);
				all_consistent = false;
			}
		}

		println!();
	}

	if all_consistent {
		println!("✅ All model lists are consistent!");
	} else {
		println!("❌ Some inconsistencies found");
		panic!("Model lists in tests don't match adapter code");
	}

	Ok(())
}
