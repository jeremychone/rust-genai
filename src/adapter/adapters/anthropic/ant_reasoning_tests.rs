use super::super::AnthropicAdapter;
use super::*;
use crate::adapter::{Adapter, AdapterKind, ServiceType};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ReasoningEffort};
use crate::resolver::AuthData;
use crate::{ModelIden, ServiceTarget};
use serde_json::json;

#[test]
fn test_anthropic_opus_4_7_uses_adaptive_thinking() {
	let chat_options = ChatOptions {
		reasoning_effort: Some(ReasoningEffort::Medium),
		..Default::default()
	};
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-4-7"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["thinking"], json!({"type": "adaptive"}));
	assert_eq!(web_req.payload["output_config"]["effort"], json!("medium"));
}

#[test]
fn test_anthropic_sonnet_5_uses_adaptive_thinking() {
	let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::High);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-5"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["thinking"], json!({"type": "adaptive"}));
	assert_eq!(web_req.payload["output_config"]["effort"], json!("high"));
}

/// `Zero` on a thinking-off-by-default model (Opus 4.7+) must emit no reasoning
/// fields at all, omitting `thinking` already means "off". See issue #251.
#[test]
fn test_anthropic_opus_4_7_reasoning_zero_omits_thinking() {
	let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Zero);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-4-7"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload.get("thinking"), None, "thinking must be omitted");
	assert_eq!(
		web_req.payload.get("output_config"),
		None,
		"output_config.effort must be omitted"
	);
}

/// `Zero` on a thinking-on-by-default model (Sonnet 5) must send the explicit
/// opt-out, since omitting the field would leave adaptive thinking on. See issue #251.
#[test]
fn test_anthropic_sonnet_5_reasoning_zero_disables_thinking() {
	let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Zero);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-5"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["thinking"], json!({"type": "disabled"}));
	assert_eq!(
		web_req.payload.get("output_config"),
		None,
		"output_config.effort must be omitted"
	);
}

/// The `-none`/`-zero` model-name suffixes must parse to `Zero` and be stripped
/// from the model name sent to the API.
#[test]
fn test_anthropic_reasoning_zero_model_name_suffix() {
	for suffix in ["none", "zero"] {
		let target = ServiceTarget {
			endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::Anthropic, format!("claude-sonnet-5-{suffix}")),
		};

		let web_req = AnthropicAdapter::to_web_request_data(
			target,
			ServiceType::Chat,
			ChatRequest::from_user("hello"),
			ChatOptionsSet::default(),
		)
		.expect("to_web_request_data should succeed");

		assert_eq!(
			web_req.payload["model"],
			json!("claude-sonnet-5"),
			"suffix -{suffix} must be stripped"
		);
		assert_eq!(web_req.payload["thinking"], json!({"type": "disabled"}));
	}
}

/// `Zero` on every other family must emit no reasoning fields: legacy models
/// (claude-sonnet-4-5), adaptive-but-off-by-default models (claude-opus-4-6), and
/// Fable/Mythos, where thinking is always-on and an explicit "disabled" is rejected,
/// so omission is the only valid payload (see `anthropic_thinking_on_by_default`).
#[test]
fn test_anthropic_reasoning_zero_other_models_omit_thinking() {
	for model in ["claude-sonnet-4-5", "claude-opus-4-6", "claude-fable-5"] {
		let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Zero);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
		let target = ServiceTarget {
			endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::Anthropic, model),
		};

		let web_req = AnthropicAdapter::to_web_request_data(
			target,
			ServiceType::Chat,
			ChatRequest::from_user("hello"),
			options_set,
		)
		.expect("to_web_request_data should succeed");

		assert_eq!(
			web_req.payload.get("thinking"),
			None,
			"thinking must be omitted for {model}"
		);
		assert_eq!(
			web_req.payload.get("output_config"),
			None,
			"output_config.effort must be omitted for {model}"
		);
	}
}

/// `claude-opus-5` carries no minor version segment. If the version comparison misses it, the
/// request falls to the legacy branch and emits `thinking.budget_tokens`, removed on this model.
#[test]
fn test_anthropic_opus_5_uses_effort_not_legacy_budget_tokens() {
	let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::High);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-5"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["output_config"]["effort"], json!("high"));
	assert_eq!(web_req.payload["thinking"], json!({"type": "adaptive"}));
}

/// A future Opus release must work without a table update, in either naming shape.
#[test]
fn test_anthropic_future_opus_generation_uses_effort() {
	for model in ["claude-opus-6", "claude-opus-5-1", "claude-opus-5-20260101"] {
		let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Medium);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
		let target = ServiceTarget {
			endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::Anthropic, model),
		};

		let web_req = AnthropicAdapter::to_web_request_data(
			target,
			ServiceType::Chat,
			ChatRequest::from_user("hello"),
			options_set,
		)
		.expect("to_web_request_data should succeed");

		assert_eq!(
			web_req.payload["output_config"]["effort"],
			json!("medium"),
			"for {model}"
		);
		assert_eq!(web_req.payload["thinking"], json!({"type": "adaptive"}), "for {model}");
	}
}

/// The optional minor segment must not promote pre-4.7 models: an absent minor reads as `0`,
/// and a date stamp (`claude-opus-4-20250514`, Opus 4.0) is not a minor version.
#[test]
fn test_anthropic_pre_4_7_models_still_use_budget_tokens() {
	for model in [
		"claude-opus-4-0",
		"claude-opus-4-1",
		"claude-opus-4-20250514",
		"claude-sonnet-4-5",
	] {
		let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::High);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
		let target = ServiceTarget {
			endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::Anthropic, model),
		};

		let web_req = AnthropicAdapter::to_web_request_data(
			target,
			ServiceType::Chat,
			ChatRequest::from_user("hello"),
			options_set,
		)
		.expect("to_web_request_data should succeed");

		assert_eq!(
			web_req.payload["thinking"],
			json!({"type": "enabled", "budget_tokens": REASONING_HIGH}),
			"for {model}"
		);
		assert_eq!(web_req.payload.get("output_config"), None, "for {model}");
	}
}

/// Fable/Mythos expose `xhigh`. Omitted from the predicate, `XHigh` fell through to the
/// `max` arm and silently bought a more expensive level than the caller asked for.
#[test]
fn test_anthropic_fable_and_mythos_support_xhigh() {
	for model in ["claude-fable-5", "claude-mythos-5"] {
		let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::XHigh);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
		let target = ServiceTarget {
			endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::Anthropic, model),
		};

		let web_req = AnthropicAdapter::to_web_request_data(
			target,
			ServiceType::Chat,
			ChatRequest::from_user("hello"),
			options_set,
		)
		.expect("to_web_request_data should succeed");

		assert_eq!(
			web_req.payload["output_config"]["effort"],
			json!("xhigh"),
			"for {model}"
		);
	}
}

/// `Zero` on Opus 5 must send the explicit opt-out: thinking is on by default there (unlike
/// Opus 4.7/4.8), so omitting the field would leave adaptive thinking running.
#[test]
fn test_anthropic_opus_5_reasoning_zero_disables_thinking() {
	let chat_options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Zero);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-5"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["thinking"], json!({"type": "disabled"}));
	assert_eq!(
		web_req.payload.get("output_config"),
		None,
		"output_config.effort must be omitted"
	);
}
