#![allow(unused)]

use std::sync::OnceLock;

fn has_model(model_prefixes: &[&str], model_name: &str) -> bool {
	model_prefixes.iter().any(|prefix| model_name.contains(prefix))
}

// NOTE: Adaptiive are opt-ins for now and can become defaults once support is broader.
// See adaptive thinking doc: https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking

fn supports_anthropic_effort(model_name: &str) -> bool {
	const SUPPORT_EFFORT_MODELS: &[&str] =
		&["claude-opus-4-6", "claude-sonnet-4-6", "claude-opus-4-5", "claude-sonnet-5"];

	has_model(SUPPORT_EFFORT_MODELS, model_name) || is_fable_or_mythos(model_name) || is_opus_4_7_or_higher(model_name)
}

fn supports_anthropic_reasoning_max(model_name: &str) -> bool {
	const SUPPORT_REASONING_MAX_MODELS: &[&str] = &["claude-opus-4-6", "claude-sonnet-5"];

	has_model(SUPPORT_REASONING_MAX_MODELS, model_name)
		|| is_fable_or_mythos(model_name)
		|| is_opus_4_7_or_higher(model_name)
}

fn supports_anthropic_reasoning_xhigh(model_name: &str) -> bool {
	is_opus_4_7_or_higher(model_name) || is_fable_or_mythos(model_name) || model_name.contains("claude-sonnet-5")
}

/// Models where Anthropic enables adaptive thinking by default when the request omits the
/// `thinking` field (currently the Claude Sonnet 5 and Claude Opus 5 families). These need an
/// explicit `{"type": "disabled"}` to turn reasoning off.
///
/// NOTE: not derivable from `is_opus_4_7_or_higher` — Opus 4.7 and 4.8 are off by default, 5 is on.
///
/// NOTE: Fable/Mythos thinking is always-on and cannot be disabled (an explicit "disabled"
/// is rejected), so they are intentionally not listed — for them, `Zero` omits `thinking`.
fn anthropic_thinking_on_by_default(model_name: &str) -> bool {
	model_name.contains("claude-sonnet-5") || model_name.contains("claude-opus-5")
}

fn supports_anthropic_adaptive_thinking(model_name: &str) -> bool {
	const SUPPORT_ADAPTIVE_THINK_MODELS: &[&str] = &["claude-opus-4-6", "claude-sonnet-4-6", "claude-sonnet-5"];

	has_model(SUPPORT_ADAPTIVE_THINK_MODELS, model_name)
		|| is_fable_or_mythos(model_name)
		|| is_opus_4_7_or_higher(model_name)
}

// endregion: --- Reasoning Support

// region:    --- Model Name Support

/// Returns true when the given model name looks like a Claude Opus model with
/// version >= 4.7 (e.g. `claude-opus-4-7`, `claude-opus-5`, ...).
///
/// The regex is unanchored and tolerates arbitrary prefixes/suffixes around the
/// core `claude-opus-<major>[-<minor>]` portion. The minor segment is optional
/// and a missing one reads as `0`. Any parse or regex failure is treated as a
/// conservative `false`.
fn is_opus_4_7_or_higher(model_name: &str) -> bool {
	/// A minor version is one or two digits; a longer run is a date stamp, not a
	/// minor. Without this, `claude-opus-4-20250514` (Opus 4.0) would read as
	/// version 4.20250514 and be promoted. Lookaround would express it in the
	/// pattern, but the `regex` crate has none.
	const MAX_MINOR_DIGITS: usize = 2;

	static RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
	let re = RE.get_or_init(|| regex::Regex::new(r"claude-opus-(\d+)(?:-(\d+))?").ok());
	let Some(re) = re.as_ref() else {
		return false;
	};
	let Some(caps) = re.captures(model_name) else {
		return false;
	};
	let major = caps.get(1).and_then(|m| m.as_str().parse::<u32>().ok());
	let minor = match caps.get(2) {
		Some(m) if m.as_str().len() > MAX_MINOR_DIGITS => Some(0),
		Some(m) => m.as_str().parse::<u32>().ok(),
		None => Some(0),
	};
	match (major, minor) {
		(Some(major), Some(minor)) => (major, minor) >= (4, 7),
		_ => false,
	}
}

fn is_fable_or_mythos(model_name: &str) -> bool {
	model_name.contains("fable") || model_name.contains("mythos")
}

// endregion: --- Model Name Support
