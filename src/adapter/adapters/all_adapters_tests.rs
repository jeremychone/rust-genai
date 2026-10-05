use super::{DeepSeekAdapter, OllamaAdapter};
use crate::adapter::AdapterDispatcher;
use crate::adapter::{Adapter, AdapterKind, ServiceType};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ReasoningEffort};
use crate::embed::{EmbedOptionsSet, EmbedRequest};
use crate::resolver::{AuthData, Endpoint};
use crate::{ModelIden, ServiceTarget};
use serde_json::Value;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_api_route_dispatch_preserves_model_and_uses_gateway_endpoint() -> Result<()> {
	for model_name in ["gpt-6.1-sol", "claude-fable-5-1", "vendor/model-id"] {
		let namespaced = format!("api_route::{model_name}");
		let kind = AdapterKind::from_model(&namespaced)?;
		assert_eq!(kind, AdapterKind::ApiRoute);
		assert_eq!(kind.default_key_env_name(), Some("API_ROUTE_API_KEY"));
		let request = AdapterDispatcher::to_web_request_data(
			ServiceTarget {
				model: ModelIden::new(kind, namespaced),
				auth: AuthData::from_single("test-route-key"),
				endpoint: AdapterDispatcher::default_endpoint(kind),
			},
			ServiceType::Chat,
			ChatRequest::from_user("hello"),
			ChatOptionsSet::default(),
		)?;
		assert_eq!(request.url, "https://global.api-route.com/v1/chat/completions");
		assert_eq!(request.payload["model"], model_name);
		assert!(
			request.headers.iter().any(|(name, value)| {
				name.eq_ignore_ascii_case("authorization") && value == "Bearer test-route-key"
			})
		);
	}
	// Unqualified model IDs keep their existing native-provider routing.
	assert_eq!(AdapterKind::from_model("claude-fable-5-1")?, AdapterKind::Anthropic);
	Ok(())
}

#[test]
fn test_heabsy_dispatch_preserves_model_and_uses_heabsy_endpoint() -> Result<()> {
	let kind = AdapterKind::from_model("heabsy::qwen38")?;
	assert_eq!(kind, AdapterKind::Heabsy);
	assert_eq!(kind.default_key_env_name(), Some("HEABSY_API_KEY"));
	let target = ServiceTarget {
		model: ModelIden::new(kind, "heabsy::qwen38"),
		auth: AuthData::from_single("test-key"),
		endpoint: AdapterDispatcher::default_endpoint(kind),
	};
	let request = AdapterDispatcher::to_web_request_data(
		target.clone(),
		ServiceType::Chat,
		ChatRequest::from_user("hello"),
		ChatOptionsSet::default(),
	)?;
	assert_eq!(request.url, "https://api.heabsy.com/v1/chat/completions");
	assert_eq!(request.payload["model"], "qwen38");
	let has_bearer = request
		.headers
		.iter()
		.any(|(name, value)| name.eq_ignore_ascii_case("authorization") && value == "Bearer test-key");
	assert!(has_bearer);
	// Heabsy does not expose an embeddings endpoint.
	let embed_res =
		AdapterDispatcher::to_embed_request_data(target, EmbedRequest::new("hello"), EmbedOptionsSet::default());
	assert!(matches!(
		embed_res,
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: AdapterKind::Heabsy,
			..
		})
	));
	Ok(())
}

// region:    --- DeepSeek

#[test]
fn test_deepseek_managed_body_thinking_enables_non_zero_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = Some(ReasoningEffort::Max);

	// -- Exec
	let payload = support_deepseek_payload(reasoning_effort)?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "enabled");
	assert_eq!(payload["reasoning_effort"], "max");

	Ok(())
}

#[test]
fn test_deepseek_managed_body_thinking_disables_zero_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = Some(ReasoningEffort::Zero);

	// -- Exec
	let payload = support_deepseek_payload(reasoning_effort)?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "disabled");
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

#[test]
fn test_deepseek_managed_body_thinking_omits_fields_without_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = None;

	// -- Exec
	let payload = support_deepseek_payload(reasoning_effort)?;

	// -- Check
	assert!(payload.get("thinking").is_none());
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

// endregion: --- DeepSeek

// region:    --- Ollama

#[test]
fn test_ollama_think_omitted_without_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = None;

	// -- Exec
	let payload = support_ollama_payload(reasoning_effort)?;

	// -- Check
	assert!(payload.get("think").is_none());

	Ok(())
}

#[test]
fn test_ollama_think_disabled_for_zero_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = Some(ReasoningEffort::Zero);

	// -- Exec
	let payload = support_ollama_payload(reasoning_effort)?;

	// -- Check
	assert_eq!(payload["think"], false);

	Ok(())
}

#[test]
fn test_ollama_think_maps_efforts_to_string_levels() -> Result<()> {
	// Note: Ollama has no "minimal" level, so Minimal maps to "low",
	//       and XHigh/Max both map to "max" (Ollama's highest level).
	for (effort, expected) in [
		(ReasoningEffort::Minimal, "low"),
		(ReasoningEffort::Low, "low"),
		(ReasoningEffort::Medium, "medium"),
		(ReasoningEffort::High, "high"),
		(ReasoningEffort::XHigh, "max"),
		(ReasoningEffort::Max, "max"),
	] {
		// -- Exec
		let payload = support_ollama_payload(Some(effort.clone()))?;

		// -- Check
		assert_eq!(payload["think"], expected, "unexpected think level for {effort:?}");
	}

	Ok(())
}

#[test]
fn test_ollama_think_enabled_for_budget_effort() -> Result<()> {
	// Note: Ollama has no token-budget knob, so Budget enables thinking at the model's default level.
	// -- Setup & Fixtures
	let reasoning_effort = Some(ReasoningEffort::Budget(1024));

	// -- Exec
	let payload = support_ollama_payload(reasoning_effort)?;

	// -- Check
	assert_eq!(payload["think"], true);

	Ok(())
}

// endregion: --- Ollama

// region:    --- Support

fn support_deepseek_payload(reasoning_effort: Option<ReasoningEffort>) -> Result<Value> {
	let chat_options = reasoning_effort.map(|effort| ChatOptions::default().with_reasoning_effort(effort));
	let options_set = ChatOptionsSet::default().with_chat_options(chat_options.as_ref());
	let request = DeepSeekAdapter::to_web_request_data(
		ServiceTarget {
			model: ModelIden::new(AdapterKind::DeepSeek, "deepseek-v4-flash"),
			auth: AuthData::from_single("test-key"),
			endpoint: Endpoint::from_static("https://api.deepseek.com/v1/"),
		},
		ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
	)?;

	Ok(request.payload)
}

fn support_ollama_payload(reasoning_effort: Option<ReasoningEffort>) -> Result<Value> {
	let chat_options = reasoning_effort.map(|effort| ChatOptions::default().with_reasoning_effort(effort));
	let options_set = ChatOptionsSet::default().with_chat_options(chat_options.as_ref());
	let request = OllamaAdapter::to_web_request_data(
		ServiceTarget {
			model: ModelIden::new(AdapterKind::Ollama, "qwen3"),
			auth: AuthData::from_single("test-key"),
			endpoint: Endpoint::from_static("http://localhost:11434/"),
		},
		ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
	)?;

	Ok(request.payload)
}

// endregion: --- Support
