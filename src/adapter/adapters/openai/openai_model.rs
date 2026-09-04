#![allow(dead_code)]

use crate::ModelName;
use crate::common::ModelIden;

// region:    --- Types

/// Represents a parsed view of an OpenAI model name.
///
/// Stores byte slice index ranges into the underlying [`ModelName`]
/// to enable zero-allocation slicing for family, variant, and snapshot getters.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::adapter) struct OpenAIModel {
	model_name: ModelName,
	family: Option<(usize, usize)>,
	version: Option<f64>,
	variant: Option<(usize, usize)>,
	snapshot: Option<(usize, usize)>,
}

// endregion: --- Types

impl OpenAIModel {
	/// Parse an `OpenAIModel` from a [`ModelName`].
	pub fn parse(model_name: ModelName) -> Self {
		let (ns_opt, name) = model_name.namespace_and_name();
		let base_offset = if let Some(ns) = ns_opt { ns.len() + 2 } else { 0 };

		if name.is_empty() {
			return Self {
				model_name,
				family: None,
				version: None,
				variant: None,
				snapshot: None,
			};
		}

		// 1. Strip trailing reasoning qualifier if present.
		let mut trimmed_name = name;
		for qualifier in ["-low", "-medium", "-high", "-max", "-none", "-zero", "-xhigh"] {
			if trimmed_name.ends_with(qualifier) {
				trimmed_name = &trimmed_name[..trimmed_name.len() - qualifier.len()];
				break;
			}
		}

		// 2. Extract trailing date snapshot if present.
		let (trimmed_name, snapshot) = extract_snapshot(trimmed_name, base_offset);

		// 3. Resolve family, version, and variant.
		let (family, version, variant) = resolve_components(trimmed_name, base_offset);

		Self {
			model_name,
			family,
			version,
			variant,
			snapshot,
		}
	}
}

// region:    --- Accessors

impl OpenAIModel {
	/// Return the underlying model name.
	pub fn model_name(&self) -> &ModelName {
		&self.model_name
	}

	/// Return the model family (e.g., `"gpt"`, `"chatgpt"`, `"o1"`, `"o3"`).
	pub fn family(&self) -> Option<&str> {
		self.family.map(|(start, end)| &self.model_name.as_str()[start..end])
	}

	/// Return the numeric version if present (e.g., `4.0`, `4.1`, `5.0`, `6.0`).
	pub fn version(&self) -> Option<f64> {
		self.version
	}

	/// Return the model variant (e.g., `"astra"`, `"mini"`, `"4o-mini"`).
	pub fn variant(&self) -> Option<&str> {
		self.variant.map(|(start, end)| &self.model_name.as_str()[start..end])
	}

	/// Return the date snapshot string if present (e.g., `"0613"`, `"2024-08-06"`).
	pub fn snapshot(&self) -> Option<&str> {
		self.snapshot.map(|(start, end)| &self.model_name.as_str()[start..end])
	}

	/// Check whether this model should route to the OpenAI Responses adapter.
	pub fn is_resp_model(&self) -> bool {
		if self.family() == Some("gpt") {
			if self.version().is_some_and(|v| v >= 5.0) {
				return true;
			}
			if let Some(variant) = self.variant()
				&& (variant.contains("codex") || variant.contains("pro"))
			{
				return true;
			}
		}
		false
	}
}

// endregion: --- Accessors

impl std::fmt::Display for OpenAIModel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.model_name)
	}
}

// region:    --- Froms

impl From<ModelName> for OpenAIModel {
	fn from(model_name: ModelName) -> Self {
		Self::parse(model_name)
	}
}

impl From<&ModelName> for OpenAIModel {
	fn from(model_name: &ModelName) -> Self {
		Self::parse(model_name.clone())
	}
}

impl From<&str> for OpenAIModel {
	fn from(s: &str) -> Self {
		Self::parse(ModelName::from(s))
	}
}

impl From<String> for OpenAIModel {
	fn from(s: String) -> Self {
		Self::parse(ModelName::from(s))
	}
}

impl From<ModelIden> for OpenAIModel {
	fn from(model_iden: ModelIden) -> Self {
		Self::parse(model_iden.model_name)
	}
}

impl From<&ModelIden> for OpenAIModel {
	fn from(model_iden: &ModelIden) -> Self {
		Self::parse(model_iden.model_name.clone())
	}
}

// endregion: --- Froms

// region:    --- Support

fn extract_snapshot(name: &str, base_offset: usize) -> (&str, Option<(usize, usize)>) {
	let bytes = name.as_bytes();

	// Pattern A: YYYY-MM-DD (10 chars preceded by hyphen)
	if name.len() >= 11 && bytes[name.len() - 11] == b'-' {
		let candidate = &name[name.len() - 10..];
		let cand_bytes = candidate.as_bytes();
		if cand_bytes[0..4].iter().all(|b| b.is_ascii_digit())
			&& cand_bytes[4] == b'-'
			&& cand_bytes[5..7].iter().all(|b| b.is_ascii_digit())
			&& cand_bytes[7] == b'-'
			&& cand_bytes[8..10].iter().all(|b| b.is_ascii_digit())
			&& let (Ok(month), Ok(day)) = (candidate[5..7].parse::<u32>(), candidate[8..10].parse::<u32>())
			&& (1..=12).contains(&month)
			&& (1..=31).contains(&day)
		{
			let start = base_offset + (name.len() - 10);
			let end = base_offset + name.len();
			let remaining = &name[..name.len() - 11];
			return (remaining, Some((start, end)));
		}
	}

	// Pattern B: YYYYMMDD (8 digits preceded by hyphen)
	if name.len() >= 9 && bytes[name.len() - 9] == b'-' {
		let candidate = &name[name.len() - 8..];
		if candidate.bytes().all(|b| b.is_ascii_digit())
			&& let (Ok(year), Ok(month), Ok(day)) = (
				candidate[0..4].parse::<u32>(),
				candidate[4..6].parse::<u32>(),
				candidate[6..8].parse::<u32>(),
			) && (1990..=2099).contains(&year)
			&& (1..=12).contains(&month)
			&& (1..=31).contains(&day)
		{
			let start = base_offset + (name.len() - 8);
			let end = base_offset + name.len();
			let remaining = &name[..name.len() - 9];
			return (remaining, Some((start, end)));
		}
	}

	// Pattern C: MMDD (4 digits preceded by hyphen)
	if name.len() >= 5 && bytes[name.len() - 5] == b'-' {
		let candidate = &name[name.len() - 4..];
		if candidate.bytes().all(|b| b.is_ascii_digit())
			&& let (Ok(month), Ok(day)) = (candidate[0..2].parse::<u32>(), candidate[2..4].parse::<u32>())
			&& (1..=12).contains(&month)
			&& (1..=31).contains(&day)
		{
			let start = base_offset + (name.len() - 4);
			let end = base_offset + name.len();
			let remaining = &name[..name.len() - 5];
			return (remaining, Some((start, end)));
		}
	}

	(name, None)
}

type ResolvedComponents = (Option<(usize, usize)>, Option<f64>, Option<(usize, usize)>);

fn resolve_components(name: &str, base_offset: usize) -> ResolvedComponents {
	// Family 1: gpt-oss
	if let Some(rest) = name.strip_prefix("gpt-oss") {
		let fam_range = (base_offset, base_offset + 7);
		let remainder = if let Some(stripped) = rest.strip_prefix('-') {
			stripped
		} else {
			rest
		};
		let variant_range = if !remainder.is_empty() {
			let rem_start = name.len() - remainder.len();
			Some((base_offset + rem_start, base_offset + name.len()))
		} else {
			None
		};
		return (Some(fam_range), None, variant_range);
	}

	// Family 2: o[n] (e.g. o1, o3, o4)
	if name.starts_with('o') && name.len() > 1 && name.as_bytes()[1].is_ascii_digit() {
		let mut digit_end = 1;
		while digit_end < name.len() && name.as_bytes()[digit_end].is_ascii_digit() {
			digit_end += 1;
		}
		if digit_end == name.len() || name.as_bytes()[digit_end] == b'-' {
			let fam_range = (base_offset, base_offset + digit_end);
			let variant_range = if digit_end < name.len() && name.as_bytes()[digit_end] == b'-' {
				let rem_start = digit_end + 1;
				if rem_start < name.len() {
					Some((base_offset + rem_start, base_offset + name.len()))
				} else {
					None
				}
			} else {
				None
			};
			return (Some(fam_range), None, variant_range);
		}
	}

	// Family 3: chatgpt
	if let Some(rest) = name.strip_prefix("chatgpt") {
		let fam_range = (base_offset, base_offset + 7);
		let (rem_str, rem_offset) = if let Some(stripped) = rest.strip_prefix('-') {
			(stripped, 8)
		} else {
			(rest, 7)
		};
		let (version, variant) = parse_version_and_variant(rem_str, base_offset + rem_offset);
		return (Some(fam_range), version, variant);
	}

	// Family 4: gpt
	if let Some(rest) = name.strip_prefix("gpt-") {
		let fam_range = (base_offset, base_offset + 3);
		let (version, variant) = parse_version_and_variant(rest, base_offset + 4);
		return (Some(fam_range), version, variant);
	} else if name == "gpt" {
		let fam_range = (base_offset, base_offset + 3);
		return (Some(fam_range), None, None);
	} else if name.starts_with("gpt") && name.len() > 3 && name.as_bytes()[3].is_ascii_digit() {
		let fam_range = (base_offset, base_offset + 3);
		let (version, variant) = parse_version_and_variant(&name[3..], base_offset + 3);
		return (Some(fam_range), version, variant);
	}

	// Family 5: Legacy completion and embedding models
	const LEGACY_PREFIXES: &[(&str, usize)] = &[
		("text-embedding", 14),
		("text-davinci", 12),
		("davinci", 7),
		("text-babbage", 12),
		("babbage", 7),
		("text-curie", 10),
		("curie", 5),
		("text-ada", 8),
		("ada", 3),
		("dall-e", 6),
		("whisper", 7),
		("tts", 3),
		("codex", 5),
	];

	for &(prefix, len) in LEGACY_PREFIXES {
		if name.starts_with(prefix) {
			let fam_range = (base_offset, base_offset + len);
			return (Some(fam_range), None, None);
		}
	}

	(None, None, None)
}

fn parse_version_and_variant(remainder: &str, remainder_offset: usize) -> (Option<f64>, Option<(usize, usize)>) {
	if remainder.is_empty() {
		return (None, None);
	}

	let first_hyphen_idx = remainder.find('-');
	let (first_token, token_remainder) = match first_hyphen_idx {
		Some(idx) => (&remainder[..idx], Some(&remainder[idx + 1..])),
		None => (remainder, None),
	};

	if is_numeric_token(first_token) {
		let version = first_token.parse::<f64>().ok();
		let variant = if let Some(rest) = token_remainder
			&& !rest.is_empty()
			&& let Some(idx) = first_hyphen_idx
		{
			let var_start = remainder_offset + idx + 1;
			let var_end = remainder_offset + remainder.len();
			Some((var_start, var_end))
		} else {
			None
		};
		(version, variant)
	} else {
		let var_start = remainder_offset;
		let var_end = remainder_offset + remainder.len();
		(None, Some((var_start, var_end)))
	}
}

fn is_numeric_token(s: &str) -> bool {
	if s.is_empty() {
		return false;
	}
	let mut has_digit = false;
	let mut dot_count = 0;
	for b in s.bytes() {
		if b.is_ascii_digit() {
			has_digit = true;
		} else if b == b'.' {
			dot_count += 1;
		} else {
			return false;
		}
	}
	has_digit && dot_count <= 1
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_adapter_openai_model_parse_gpt6_astra() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "gpt-6-astra";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("gpt"));
		assert_eq!(model.version(), Some(6.0));
		assert_eq!(model.variant(), Some("astra"));
		assert_eq!(model.snapshot(), None);
		assert!(model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_gpt5() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "gpt-5";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("gpt"));
		assert_eq!(model.version(), Some(5.0));
		assert_eq!(model.variant(), None);
		assert_eq!(model.snapshot(), None);
		assert!(model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_gpt4o_mini() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "gpt-4o-mini";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("gpt"));
		assert_eq!(model.version(), None);
		assert_eq!(model.variant(), Some("4o-mini"));
		assert_eq!(model.snapshot(), None);
		assert!(!model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_gpt4_snapshot() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "gpt-4-0613";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("gpt"));
		assert_eq!(model.version(), Some(4.0));
		assert_eq!(model.variant(), None);
		assert_eq!(model.snapshot(), Some("0613"));
		assert!(!model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_gpt4o_date_snapshot() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "gpt-4o-2024-08-06";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("gpt"));
		assert_eq!(model.version(), None);
		assert_eq!(model.variant(), Some("4o"));
		assert_eq!(model.snapshot(), Some("2024-08-06"));
		assert!(!model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_o_series_qualifier() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "o3-mini-2025-01-31-high";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("o3"));
		assert_eq!(model.version(), None);
		assert_eq!(model.variant(), Some("mini"));
		assert_eq!(model.snapshot(), Some("2025-01-31"));
		assert!(!model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_gpt_oss() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "gpt-oss-120b";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("gpt-oss"));
		assert_eq!(model.version(), None);
		assert_eq!(model.variant(), Some("120b"));
		assert_eq!(model.snapshot(), None);
		assert!(!model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_namespaced() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "openai::gpt-6-astra";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.model_name().namespace(), Some("openai"));
		assert_eq!(model.family(), Some("gpt"));
		assert_eq!(model.version(), Some(6.0));
		assert_eq!(model.variant(), Some("astra"));
		assert_eq!(model.snapshot(), None);
		assert!(model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_parse_legacy_embedding() -> Result<()> {
		// -- Setup & Fixtures
		let model_str = "text-embedding-3-small";

		// -- Exec
		let model = OpenAIModel::from(model_str);

		// -- Check
		assert_eq!(model.family(), Some("text-embedding"));
		assert_eq!(model.version(), None);
		assert_eq!(model.variant(), None);
		assert_eq!(model.snapshot(), None);
		assert!(!model.is_resp_model());

		Ok(())
	}

	#[test]
	fn test_adapter_openai_model_is_resp_model_variants() -> Result<()> {
		// -- Setup & Fixtures
		let gpt4o_codex = OpenAIModel::from("gpt-4o-codex");
		let gpt4o_pro = OpenAIModel::from("gpt-4o-pro");
		let codex_standalone = OpenAIModel::from("codex-cushman-001");
		let chatgpt_latest = OpenAIModel::from("chatgpt-4o-latest");

		// -- Exec & Check
		assert!(gpt4o_codex.is_resp_model());
		assert!(gpt4o_pro.is_resp_model());
		assert!(!codex_standalone.is_resp_model());
		assert!(!chatgpt_latest.is_resp_model());

		Ok(())
	}
}

// endregion: --- Tests
