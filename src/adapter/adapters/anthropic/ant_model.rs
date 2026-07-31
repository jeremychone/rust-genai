#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnthropicModelFamily {
	Opus,
	Sonnet,
	Haiku,
	Fable,
	Mythos,
	Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnthropicModel<'a> {
	pub normalized_name: &'a str,
	pub family: AnthropicModelFamily,
	pub major_version: Option<u32>,
	pub minor_version: Option<u32>,
	pub date_label: Option<&'a str>,
	pub remaining_suffix: Vec<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AnthropicModelCapabilities {
	pub supports_effort: bool,
	pub supports_max_effort: bool,
	pub supports_xhigh_effort: bool,
	pub supports_adaptive_thinking: bool,
	pub thinking_enabled_by_default: bool,
	pub supports_legacy_budget_thinking: bool,
	pub max_tokens: AnthropicMaxTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnthropicMaxTokens {
	Tokens4K,
	Tokens8K,
	Tokens32K,
	Tokens64K,
	Tokens128K,
}

impl<'a> AnthropicModel<'a> {
	pub fn parse(normalized_name: &'a str) -> Self {
		let segments = normalized_name.split('-').collect::<Vec<_>>();
		if segments.first() != Some(&"claude") {
			return Self::unknown(normalized_name);
		}

		let Some(family_name) = segments.get(1) else {
			return Self::unknown(normalized_name);
		};
		let family = AnthropicModelFamily::from_segment(family_name);
		if family == AnthropicModelFamily::Unknown {
			return Self::unknown(normalized_name);
		}

		let model_segments = segments.get(2..).unwrap_or_default();
		let mut segment_index = 0;
		let mut major_version = None;
		let mut minor_version = None;
		let mut date_label = None;

		if let Some(segment) = model_segments.get(segment_index) {
			if is_version_segment(segment) {
				major_version = segment.parse().ok();
				segment_index += 1;
			} else if is_date_label(segment) {
				date_label = Some(*segment);
				segment_index += 1;
			}
		}

		if major_version.is_some()
			&& let Some(segment) = model_segments.get(segment_index)
			&& is_version_segment(segment)
		{
			minor_version = segment.parse().ok();
			segment_index += 1;
		}

		if date_label.is_none()
			&& let Some(segment) = model_segments.get(segment_index)
			&& is_date_label(segment)
		{
			date_label = Some(*segment);
			segment_index += 1;
		}

		Self {
			normalized_name,
			family,
			major_version,
			minor_version,
			date_label,
			remaining_suffix: model_segments[segment_index..].to_vec(),
		}
	}

	pub fn version(&self) -> Option<(u32, u32)> {
		self.major_version.map(|major| (major, self.minor_version.unwrap_or(0)))
	}

	pub fn capabilities(&self) -> AnthropicModelCapabilities {
		let supports_effort = self.supports_effort();
		let supports_max_effort = self.supports_max_effort();
		let supports_xhigh_effort = self.supports_xhigh_effort();
		let supports_adaptive_thinking = self.supports_adaptive_thinking();

		AnthropicModelCapabilities {
			supports_effort,
			supports_max_effort,
			supports_xhigh_effort,
			supports_adaptive_thinking,
			thinking_enabled_by_default: self.thinking_enabled_by_default(),
			supports_legacy_budget_thinking: !supports_adaptive_thinking,
			max_tokens: max_tokens_for_name(self.normalized_name),
		}
	}

	fn unknown(normalized_name: &'a str) -> Self {
		Self {
			normalized_name,
			family: AnthropicModelFamily::Unknown,
			major_version: None,
			minor_version: None,
			date_label: None,
			remaining_suffix: Vec::new(),
		}
	}

	fn supports_effort(&self) -> bool {
		match self.family {
			AnthropicModelFamily::Opus => self.version().is_some_and(|version| version >= (4, 5)),
			AnthropicModelFamily::Sonnet => matches!(self.version(), Some((4, 6) | (5, _))),
			AnthropicModelFamily::Fable | AnthropicModelFamily::Mythos => true,
			AnthropicModelFamily::Haiku => false,
			AnthropicModelFamily::Unknown => legacy_supports_effort(self.normalized_name),
		}
	}

	fn supports_max_effort(&self) -> bool {
		match self.family {
			AnthropicModelFamily::Opus => self.version().is_some_and(|version| version >= (4, 6)),
			AnthropicModelFamily::Sonnet => matches!(self.version(), Some((4, 6) | (5, _))),
			AnthropicModelFamily::Fable | AnthropicModelFamily::Mythos => true,
			AnthropicModelFamily::Haiku => false,
			AnthropicModelFamily::Unknown => legacy_supports_max_effort(self.normalized_name),
		}
	}

	fn supports_xhigh_effort(&self) -> bool {
		match self.family {
			AnthropicModelFamily::Opus => self.version().is_some_and(|version| version >= (4, 7)),
			AnthropicModelFamily::Sonnet => matches!(self.version(), Some((5, _))),
			AnthropicModelFamily::Fable | AnthropicModelFamily::Mythos => true,
			AnthropicModelFamily::Haiku => false,
			AnthropicModelFamily::Unknown => legacy_supports_xhigh_effort(self.normalized_name),
		}
	}

	fn supports_adaptive_thinking(&self) -> bool {
		match self.family {
			AnthropicModelFamily::Opus => self.version().is_some_and(|version| version >= (4, 6)),
			AnthropicModelFamily::Sonnet => matches!(self.version(), Some((4, 6) | (5, _))),
			AnthropicModelFamily::Fable | AnthropicModelFamily::Mythos => true,
			AnthropicModelFamily::Haiku => false,
			AnthropicModelFamily::Unknown => legacy_supports_adaptive_thinking(self.normalized_name),
		}
	}

	fn thinking_enabled_by_default(&self) -> bool {
		match self.family {
			AnthropicModelFamily::Opus | AnthropicModelFamily::Sonnet => {
				matches!(self.version(), Some((5, _)))
			}
			AnthropicModelFamily::Haiku | AnthropicModelFamily::Fable | AnthropicModelFamily::Mythos => false,
			AnthropicModelFamily::Unknown => legacy_thinking_enabled_by_default(self.normalized_name),
		}
	}
}

impl AnthropicModelFamily {
	fn from_segment(segment: &str) -> Self {
		match segment {
			"opus" => Self::Opus,
			"sonnet" => Self::Sonnet,
			"haiku" => Self::Haiku,
			"fable" => Self::Fable,
			"mythos" => Self::Mythos,
			_ => Self::Unknown,
		}
	}
}

// region:    --- Support

fn is_version_segment(segment: &str) -> bool {
	(1..=2).contains(&segment.len()) && segment.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_date_label(segment: &str) -> bool {
	segment.len() == 8 && segment.bytes().all(|byte| byte.is_ascii_digit())
}

fn legacy_opus_version(model_name: &str) -> Option<(u32, u32)> {
	let version = model_name.split_once("claude-opus-")?.1;
	let major_len = version.bytes().take_while(u8::is_ascii_digit).count();
	let major = version.get(..major_len)?.parse().ok()?;
	let remaining = version.get(major_len..)?;
	let minor_digits = remaining
		.strip_prefix('-')
		.map(|minor| minor.bytes().take_while(u8::is_ascii_digit).count())
		.unwrap_or(0);
	let minor = if minor_digits == 0 || minor_digits > 2 {
		0
	} else {
		remaining.get(1..=minor_digits)?.parse().ok()?
	};
	Some((major, minor))
}

fn legacy_is_fable_or_mythos(model_name: &str) -> bool {
	model_name.contains("fable") || model_name.contains("mythos")
}

fn legacy_supports_effort(model_name: &str) -> bool {
	const SUPPORT_EFFORT_MODELS: &[&str] =
		&["claude-opus-4-6", "claude-sonnet-4-6", "claude-opus-4-5", "claude-sonnet-5"];

	SUPPORT_EFFORT_MODELS.iter().any(|name| model_name.contains(name))
		|| legacy_is_fable_or_mythos(model_name)
		|| legacy_opus_version(model_name).is_some_and(|version| version >= (4, 7))
}

fn legacy_supports_max_effort(model_name: &str) -> bool {
	const SUPPORT_MAX_MODELS: &[&str] = &["claude-opus-4-6", "claude-sonnet-5"];

	SUPPORT_MAX_MODELS.iter().any(|name| model_name.contains(name))
		|| legacy_is_fable_or_mythos(model_name)
		|| legacy_opus_version(model_name).is_some_and(|version| version >= (4, 7))
}

fn legacy_supports_xhigh_effort(model_name: &str) -> bool {
	legacy_opus_version(model_name).is_some_and(|version| version >= (4, 7))
		|| legacy_is_fable_or_mythos(model_name)
		|| model_name.contains("claude-sonnet-5")
}

fn legacy_supports_adaptive_thinking(model_name: &str) -> bool {
	const SUPPORT_ADAPTIVE_MODELS: &[&str] = &["claude-opus-4-6", "claude-sonnet-4-6", "claude-sonnet-5"];

	SUPPORT_ADAPTIVE_MODELS.iter().any(|name| model_name.contains(name))
		|| legacy_is_fable_or_mythos(model_name)
		|| legacy_opus_version(model_name).is_some_and(|version| version >= (4, 7))
}

fn legacy_thinking_enabled_by_default(model_name: &str) -> bool {
	model_name.contains("claude-sonnet-5") || model_name.contains("claude-opus-5")
}

fn max_tokens_for_name(model_name: &str) -> AnthropicMaxTokens {
	if legacy_is_fable_or_mythos(model_name) {
		AnthropicMaxTokens::Tokens128K
	} else if model_name.contains("claude-sonnet")
		|| model_name.contains("claude-haiku")
		|| model_name.contains("claude-3-7-sonnet")
		|| model_name.contains("claude-opus-4-5")
	{
		AnthropicMaxTokens::Tokens64K
	} else if model_name.contains("claude-opus-4") {
		AnthropicMaxTokens::Tokens32K
	} else if model_name.contains("claude-3-5") {
		AnthropicMaxTokens::Tokens8K
	} else if model_name.contains("3-opus") || model_name.contains("3-haiku") {
		AnthropicMaxTokens::Tokens4K
	} else {
		AnthropicMaxTokens::Tokens64K
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
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
	fn test_anthropic_model_max_tokens_capability_preserves_existing_classes() -> Result<()> {
		// -- Setup & Fixtures
		let cases = [
			("claude-fable-5", AnthropicMaxTokens::Tokens128K),
			("claude-sonnet-4-6", AnthropicMaxTokens::Tokens64K),
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
}

// endregion: --- Tests
