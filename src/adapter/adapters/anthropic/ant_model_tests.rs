type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;

#[test]
fn test_anthropic_model_parse_known_models() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("claude-opus-4-7", AnthropicModelFamily::Opus, Some((4, 7)), None),
		(
			"claude-opus-4-8-20260701",
			AnthropicModelFamily::Opus,
			Some((4, 8)),
			Some("20260701"),
		),
		(
			"claude-opus-4-20250514",
			AnthropicModelFamily::Opus,
			Some((4, 0)),
			Some("20250514"),
		),
		("claude-opus-5", AnthropicModelFamily::Opus, Some((5, 0)), None),
		("claude-sonnet-4-6", AnthropicModelFamily::Sonnet, Some((4, 6)), None),
		("claude-haiku-4-5", AnthropicModelFamily::Haiku, Some((4, 5)), None),
		("claude-fable-5", AnthropicModelFamily::Fable, Some((5, 0)), None),
		("claude-mythos-5", AnthropicModelFamily::Mythos, Some((5, 0)), None),
	];

	// -- Exec & Check
	for (name, expected_family, expected_version, expected_date) in cases {
		let model = AnthropicModel::parse(name);
		assert_eq!(model.normalized_name, name);
		assert_eq!(model.family, expected_family, "unexpected family for {name}");
		assert_eq!(model.version(), expected_version, "unexpected version for {name}");
		assert_eq!(model.date_label, expected_date, "unexpected date for {name}");
		assert!(model.remaining_suffix.is_empty(), "unexpected suffix for {name}");
	}

	Ok(())
}

#[test]
fn test_anthropic_model_parse_latest_alias() -> Result<()> {
	// -- Setup & Fixtures
	let name = "claude-opus-4-6-latest";

	// -- Exec
	let model = AnthropicModel::parse(name);

	// -- Check
	assert_eq!(model.family, AnthropicModelFamily::Opus);
	assert_eq!(model.version(), Some((4, 6)));
	assert_eq!(model.date_label, None);
	assert_eq!(model.remaining_suffix, ["latest"]);

	Ok(())
}

#[test]
fn test_anthropic_model_parse_malformed_numeric_segments() -> Result<()> {
	// -- Setup & Fixtures
	let name = "claude-opus-four-7-preview";

	// -- Exec
	let model = AnthropicModel::parse(name);

	// -- Check
	assert_eq!(model.normalized_name, name);
	assert_eq!(model.family, AnthropicModelFamily::Opus);
	assert_eq!(model.version(), None);
	assert_eq!(model.date_label, None);
	assert_eq!(model.remaining_suffix, ["four", "7", "preview"]);

	Ok(())
}

#[test]
fn test_anthropic_model_parse_unrelated_custom_name() -> Result<()> {
	// -- Setup & Fixtures
	let name = "custom-claude-opus-4-7";

	// -- Exec
	let model = AnthropicModel::parse(name);

	// -- Check
	assert_eq!(model.normalized_name, name);
	assert_eq!(model.family, AnthropicModelFamily::Unknown);
	assert_eq!(model.version(), None);
	assert_eq!(model.date_label, None);
	assert!(model.remaining_suffix.is_empty());

	Ok(())
}

#[test]
fn test_anthropic_model_parse_unknown_family() -> Result<()> {
	// -- Setup & Fixtures
	let name = "claude-unrecognized-5-preview";

	// -- Exec
	let model = AnthropicModel::parse(name);

	// -- Check
	assert_eq!(model.normalized_name, name);
	assert_eq!(model.family, AnthropicModelFamily::Unknown);
	assert_eq!(model.version(), None);

	Ok(())
}

#[test]
fn test_anthropic_model_capability_matrix() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("claude-opus-4-5", true, false, false, false, false, true),
		("claude-opus-4-6", true, true, false, true, false, false),
		("claude-opus-4-7", true, true, true, true, false, false),
		("claude-opus-4-8", true, true, true, true, false, false),
		("claude-opus-5", true, true, true, true, true, false),
		("claude-sonnet-4-6", true, true, false, true, false, false),
		("claude-sonnet-5", true, true, true, true, true, false),
		("claude-fable-5", true, true, true, true, false, false),
		("claude-mythos-5", true, true, true, true, false, false),
		("claude-haiku-4-5", false, false, false, false, false, true),
		("claude-haiku-5-5", true, true, true, true, true, false),
	];

	// -- Exec & Check
	for (name, effort, max, xhigh, adaptive, default_thinking, legacy_budget) in cases {
		let capabilities = AnthropicModel::parse(name).capabilities();
		assert_eq!(capabilities.supports_effort, effort, "effort for {name}");
		assert_eq!(capabilities.supports_max_effort, max, "max for {name}");
		assert_eq!(capabilities.supports_xhigh_effort, xhigh, "xhigh for {name}");
		assert_eq!(
			capabilities.supports_adaptive_thinking, adaptive,
			"adaptive thinking for {name}"
		);
		assert_eq!(
			capabilities.thinking_enabled_by_default, default_thinking,
			"default thinking for {name}"
		);
		assert_eq!(
			capabilities.supports_legacy_budget_thinking, legacy_budget,
			"legacy budget thinking for {name}"
		);
	}

	Ok(())
}

#[test]
fn test_anthropic_model_capabilities_preserve_unknown_and_custom_behavior() -> Result<()> {
	// -- Setup & Fixtures
	let custom = AnthropicModel::parse("custom-claude-opus-4-7-preview");
	let unknown = AnthropicModel::parse("unrecognized-model");

	// -- Exec
	let custom_capabilities = custom.capabilities();
	let unknown_capabilities = unknown.capabilities();

	// -- Check
	assert_eq!(custom.family, AnthropicModelFamily::Unknown);
	assert!(custom_capabilities.supports_effort);
	assert!(custom_capabilities.supports_max_effort);
	assert!(custom_capabilities.supports_xhigh_effort);
	assert!(custom_capabilities.supports_adaptive_thinking);
	assert!(!unknown_capabilities.supports_effort);
	assert!(!unknown_capabilities.supports_adaptive_thinking);
	assert!(unknown_capabilities.supports_legacy_budget_thinking);

	Ok(())
}

#[test]
fn test_anthropic_model_parse_segment_boundaries() -> Result<()> {
	let cases = [
		("", AnthropicModelFamily::Unknown, None, None),
		("claude", AnthropicModelFamily::Unknown, None, None),
		("claude-opus", AnthropicModelFamily::Opus, None, None),
		("claude-opus-4", AnthropicModelFamily::Opus, Some((4, 0)), None),
		("claude-opus-4-7", AnthropicModelFamily::Opus, Some((4, 7)), None),
		(
			"claude-opus-4-7-20260701",
			AnthropicModelFamily::Opus,
			Some((4, 7)),
			Some("20260701"),
		),
		(
			"claude-opus-20260701",
			AnthropicModelFamily::Opus,
			None,
			Some("20260701"),
		),
	];

	for (name, expected_family, expected_version, expected_date) in cases {
		let model = AnthropicModel::parse(name);
		assert_eq!(model.normalized_name, name);
		assert_eq!(model.family, expected_family, "unexpected family for {name}");
		assert_eq!(model.version(), expected_version, "unexpected version for {name}");
		assert_eq!(model.date_label, expected_date, "unexpected date for {name}");
		assert!(model.remaining_suffix.is_empty(), "unexpected suffix for {name}");
	}

	Ok(())
}

#[test]
fn test_anthropic_model_max_tokens_capability_preserves_existing_classes() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("claude-fable-5", AnthropicMaxTokens::Tokens128K),
		("claude-sonnet-4-6", AnthropicMaxTokens::Tokens64K),
		("claude-haiku-4-5", AnthropicMaxTokens::Tokens64K),
		("claude-haiku-5-5", AnthropicMaxTokens::Tokens128K),
		("claude-opus-4-5", AnthropicMaxTokens::Tokens64K),
		("claude-opus-4-0", AnthropicMaxTokens::Tokens32K),
		("claude-3-5-sonnet", AnthropicMaxTokens::Tokens8K),
		("claude-3-opus-20240229", AnthropicMaxTokens::Tokens4K),
		("unrecognized-model", AnthropicMaxTokens::Tokens64K),
		("custom-fable-alias", AnthropicMaxTokens::Tokens128K),
		("vendor-claude-opus-4-custom", AnthropicMaxTokens::Tokens32K),
	];

	// -- Exec & Check
	for (name, expected) in cases {
		assert_eq!(
			AnthropicModel::parse(name).capabilities().max_tokens,
			expected,
			"max-token class for {name}"
		);
	}

	Ok(())
}
